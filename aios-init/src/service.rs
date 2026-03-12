use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use tracing::{error, info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub auto_start: bool,
    pub restart_on_failure: bool,
    pub max_restarts: u32,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub environment: HashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServiceState {
    Stopped,
    Starting,
    Running,
    Failed,
    Restarting,
}

impl std::fmt::Display for ServiceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServiceState::Stopped => write!(f, "stopped"),
            ServiceState::Starting => write!(f, "starting"),
            ServiceState::Running => write!(f, "running"),
            ServiceState::Failed => write!(f, "failed"),
            ServiceState::Restarting => write!(f, "restarting"),
        }
    }
}

pub struct ServiceInstance {
    pub config: ServiceConfig,
    pub state: ServiceState,
    pub process: Option<Child>,
    pub restart_count: u32,
    pub pid: Option<u32>,
}

pub struct ServiceManager {
    services: HashMap<String, ServiceInstance>,
    start_order: Vec<String>,
}

impl ServiceManager {
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
            start_order: Vec::new(),
        }
    }

    pub fn register(&mut self, config: ServiceConfig) {
        let name = config.name.clone();
        self.services.insert(
            name.clone(),
            ServiceInstance {
                config,
                state: ServiceState::Stopped,
                process: None,
                restart_count: 0,
                pid: None,
            },
        );
        if !self.start_order.contains(&name) {
            self.start_order.push(name);
        }
    }

    pub fn resolve_start_order(&mut self) {
        let mut order = Vec::new();
        let mut visited = std::collections::HashSet::new();

        let configs: HashMap<String, Vec<String>> = self
            .services
            .iter()
            .map(|(k, v)| (k.clone(), v.config.depends_on.clone()))
            .collect();

        fn visit(
            name: &str,
            configs: &HashMap<String, Vec<String>>,
            visited: &mut std::collections::HashSet<String>,
            order: &mut Vec<String>,
        ) {
            if visited.contains(name) {
                return;
            }
            visited.insert(name.to_string());
            if let Some(deps) = configs.get(name) {
                for dep in deps {
                    visit(dep, configs, visited, order);
                }
            }
            order.push(name.to_string());
        }

        for name in self.services.keys() {
            visit(name, &configs, &mut visited, &mut order);
        }

        self.start_order = order;
    }

    pub fn start_service(&mut self, name: &str) -> Result<(), String> {
        let instance = self
            .services
            .get_mut(name)
            .ok_or_else(|| format!("Service '{}' not found", name))?;

        if instance.state == ServiceState::Running {
            return Ok(());
        }

        info!(service = name, "starting service");
        instance.state = ServiceState::Starting;

        let mut cmd = Command::new(&instance.config.command);
        cmd.args(&instance.config.args);
        cmd.envs(&instance.config.environment);
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::null());

        match cmd.spawn() {
            Ok(child) => {
                instance.pid = Some(child.id());
                instance.process = Some(child);
                instance.state = ServiceState::Running;
                info!(service = name, pid = ?instance.pid, "service started");
                Ok(())
            }
            Err(e) => {
                instance.state = ServiceState::Failed;
                error!(service = name, error = %e, "failed to start service");
                Err(format!("Failed to start {}: {}", name, e))
            }
        }
    }

    pub fn stop_service(&mut self, name: &str) -> Result<(), String> {
        let instance = self
            .services
            .get_mut(name)
            .ok_or_else(|| format!("Service '{}' not found", name))?;

        if instance.state != ServiceState::Running {
            return Ok(());
        }

        info!(service = name, "stopping service");
        if let Some(ref mut child) = instance.process {
            let _ = child.kill();
            let _ = child.wait();
        }
        instance.process = None;
        instance.pid = None;
        instance.state = ServiceState::Stopped;
        Ok(())
    }

    pub fn start_all_auto(&mut self) {
        self.resolve_start_order();
        let order = self.start_order.clone();
        for name in &order {
            if let Some(instance) = self.services.get(name) {
                if instance.config.auto_start {
                    if let Err(e) = self.start_service(name) {
                        warn!(service = name, error = %e, "auto-start failed");
                    }
                }
            }
        }
    }

    pub fn stop_all(&mut self) {
        let order: Vec<String> = self.start_order.iter().rev().cloned().collect();
        for name in &order {
            if let Err(e) = self.stop_service(name) {
                warn!(service = name, error = %e, "stop failed");
            }
        }
    }

    pub fn check_services(&mut self) {
        let names: Vec<String> = self.services.keys().cloned().collect();
        for name in names {
            let instance = self.services.get_mut(&name).unwrap();
            if instance.state != ServiceState::Running {
                continue;
            }

            let exited = if let Some(ref mut child) = instance.process {
                matches!(child.try_wait(), Ok(Some(_)))
            } else {
                false
            };

            if exited {
                warn!(service = name, "service exited unexpectedly");
                instance.process = None;
                instance.pid = None;

                if instance.config.restart_on_failure
                    && instance.restart_count < instance.config.max_restarts
                {
                    instance.state = ServiceState::Restarting;
                    instance.restart_count += 1;
                    info!(
                        service = name,
                        attempt = instance.restart_count,
                        "restarting service"
                    );
                    let _ = self.start_service(&name);
                } else {
                    instance.state = ServiceState::Failed;
                    error!(service = name, "service failed permanently");
                }
            }
        }
    }

    pub fn status(&self) -> Vec<(String, String, Option<u32>)> {
        self.services
            .iter()
            .map(|(name, instance)| {
                (
                    name.clone(),
                    instance.state.to_string(),
                    instance.pid,
                )
            })
            .collect()
    }
}
