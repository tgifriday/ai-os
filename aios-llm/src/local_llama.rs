//! In-process GGUF inference backend powered by an embedded llama.cpp
//! (`llama-cpp-2`). Compiled only when the `local-inference` feature is enabled.
//!
//! llama.cpp objects (backend/model/context) are not `Send`, and the
//! `LlmBackend` trait is async and `Send + Sync`, so the model is owned by a
//! dedicated worker OS thread. Requests are sent over a channel and answered
//! via a oneshot, keeping the model resident (loaded once) while remaining
//! sound across `.await` points.

use std::num::NonZeroU32;
use std::path::PathBuf;
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, OnceLock};
use std::thread;

use anyhow::{anyhow, Context, Result};
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel, Special};
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::{send_logs_to_tracing, LogOptions};
use tokio::sync::oneshot;

use crate::config::LocalConfig;

struct GenJob {
    prompt: String,
    max_tokens: usize,
    reply: oneshot::Sender<Result<String>>,
}

/// Handle to the worker thread that owns the loaded model.
pub struct LocalEngine {
    tx: std_mpsc::Sender<GenJob>,
    model_label: String,
}

impl LocalEngine {
    pub fn model_label(&self) -> &str {
        &self.model_label
    }

    fn spawn(model_path: PathBuf, threads: u32, n_ctx: u32) -> Result<Self> {
        let (job_tx, job_rx) = std_mpsc::channel::<GenJob>();
        let (ready_tx, ready_rx) = std_mpsc::channel::<Result<(), String>>();

        let model_label = model_path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| model_path.display().to_string());
        let path_for_thread = model_path.clone();

        thread::Builder::new()
            .name("aios-local-llm".to_string())
            .spawn(move || {
                // Route llama.cpp's chatty logs into tracing, disabled.
                send_logs_to_tracing(LogOptions::default().with_logs_enabled(false));

                let init = (|| -> Result<(LlamaBackend, LlamaModel)> {
                    let mut backend = LlamaBackend::init().context("initializing llama backend")?;
                    backend.void_logs();
                    let model_params = LlamaModelParams::default();
                    let model =
                        LlamaModel::load_from_file(&backend, &path_for_thread, &model_params)
                            .with_context(|| {
                                format!("loading GGUF model at {}", path_for_thread.display())
                            })?;
                    Ok((backend, model))
                })();

                match init {
                    Err(e) => {
                        let _ = ready_tx.send(Err(format!("{e:#}")));
                    }
                    Ok((backend, model)) => {
                        let _ = ready_tx.send(Ok(()));
                        worker_loop(&backend, &model, threads, n_ctx, job_rx);
                    }
                }
            })
            .context("spawning local LLM worker thread")?;

        match ready_rx.recv() {
            Ok(Ok(())) => Ok(Self {
                tx: job_tx,
                model_label,
            }),
            Ok(Err(e)) => Err(anyhow!("local model failed to load: {e}")),
            Err(_) => Err(anyhow!("local LLM worker exited during startup")),
        }
    }

    /// Generate a completion for a fully-formatted prompt. Awaitable and
    /// `Send`-safe: only the prompt/response strings cross the thread boundary.
    pub async fn generate(&self, prompt: String, max_tokens: usize) -> Result<String> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(GenJob {
                prompt,
                max_tokens,
                reply: reply_tx,
            })
            .map_err(|_| anyhow!("local LLM worker is not running"))?;
        reply_rx
            .await
            .map_err(|_| anyhow!("local LLM worker dropped the request"))?
    }
}

fn worker_loop(
    backend: &LlamaBackend,
    model: &LlamaModel,
    threads: u32,
    n_ctx: u32,
    rx: std_mpsc::Receiver<GenJob>,
) {
    while let Ok(job) = rx.recv() {
        let result = run_one(backend, model, threads, n_ctx, &job.prompt, job.max_tokens);
        let _ = job.reply.send(result);
    }
}

fn run_one(
    backend: &LlamaBackend,
    model: &LlamaModel,
    threads: u32,
    n_ctx: u32,
    prompt: &str,
    max_tokens: usize,
) -> Result<String> {
    let n_threads = threads.max(1) as i32;
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(n_ctx))
        .with_n_threads(n_threads)
        .with_n_threads_batch(n_threads);
    let mut ctx = model
        .new_context(backend, ctx_params)
        .context("creating llama context")?;

    let tokens = model
        .str_to_token(prompt, AddBos::Never)
        .map_err(|e| anyhow!("tokenizing prompt: {e}"))?;
    let n_ctx_usize = n_ctx as usize;
    if tokens.len() >= n_ctx_usize {
        return Err(anyhow!(
            "prompt ({} tokens) exceeds local context window ({})",
            tokens.len(),
            n_ctx
        ));
    }

    let mut batch = LlamaBatch::new(tokens.len().max(512), 1);
    let last_idx = tokens.len().saturating_sub(1);
    for (i, token) in tokens.iter().enumerate() {
        batch
            .add(*token, i as i32, &[0], i == last_idx)
            .map_err(|e| anyhow!("adding prompt token to batch: {e}"))?;
    }
    ctx.decode(&mut batch)
        .map_err(|e| anyhow!("decoding prompt: {e}"))?;

    let mut sampler = LlamaSampler::greedy();

    let mut collected: Vec<u8> = Vec::new();
    let mut n_cur = batch.n_tokens();
    let mut decoded = 0usize;

    loop {
        let token = sampler.sample(&ctx, batch.n_tokens() - 1);
        sampler.accept(token);

        if model.is_eog_token(token) {
            break;
        }

        #[allow(deprecated)]
        let bytes = model
            .token_to_bytes(token, Special::Plaintext)
            .map_err(|e| anyhow!("detokenizing: {e}"))?;
        collected.extend_from_slice(&bytes);

        decoded += 1;
        if decoded >= max_tokens || (n_cur as usize) >= n_ctx_usize {
            break;
        }

        batch.clear();
        batch
            .add(token, n_cur, &[0], true)
            .map_err(|e| anyhow!("adding generated token to batch: {e}"))?;
        n_cur += 1;
        ctx.decode(&mut batch)
            .map_err(|e| anyhow!("decoding generated token: {e}"))?;
    }

    Ok(String::from_utf8_lossy(&collected).trim().to_string())
}

/// Process-wide engine. llama.cpp's backend may only be initialized once per
/// process, so the first configured model wins; changing the local model
/// requires restarting the shell.
static ENGINE: OnceLock<std::result::Result<Arc<LocalEngine>, String>> = OnceLock::new();

/// Lazily start (or reuse) the local inference engine for the configured model.
pub fn get_or_init_engine(config: &LocalConfig) -> Result<Arc<LocalEngine>> {
    let cell = ENGINE.get_or_init(|| {
        let path = crate::local::expand_tilde(&config.model_path);
        LocalEngine::spawn(path, config.threads, config.n_ctx)
            .map(Arc::new)
            .map_err(|e| format!("{e:#}"))
    });
    cell.clone().map_err(|e| anyhow!(e))
}
