use crate::store::{Document, KnowledgeStore, SearchResult};

pub struct KnowledgeIndex {
    store: KnowledgeStore,
}

impl KnowledgeIndex {
    pub fn new() -> Self {
        let mut idx = Self {
            store: KnowledgeStore::new(),
        };
        idx.populate_builtin_docs();
        idx
    }

    pub fn store(&self) -> &KnowledgeStore {
        &self.store
    }

    pub fn store_mut(&mut self) -> &mut KnowledgeStore {
        &mut self.store
    }

    pub fn query(&self, query: &str, limit: usize) -> Vec<SearchResult> {
        self.store.search(query, limit)
    }

    pub fn query_for_context(&self, query: &str) -> String {
        let results = self.store.search(query, 3);
        if results.is_empty() {
            return String::new();
        }

        let mut out = String::from("## Relevant Knowledge\n\n");
        for r in &results {
            out.push_str(&format!("### {}\n{}\n\n", r.document.title, r.document.content));
        }
        out
    }

    fn populate_builtin_docs(&mut self) {
        self.add_commands();
        self.add_concepts();
    }

    fn cmd(&mut self, id: &str, title: &str, content: &str, tags: &[&str]) {
        self.store.add_document(Document {
            id: format!("cmd-{id}"),
            title: title.to_string(),
            content: content.to_string(),
            category: "command".to_string(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
        });
    }

    fn concept(&mut self, id: &str, title: &str, content: &str, tags: &[&str]) {
        self.store.add_document(Document {
            id: format!("concept-{id}"),
            title: title.to_string(),
            content: content.to_string(),
            category: "concept".to_string(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
        });
    }

    fn add_commands(&mut self) {
        self.cmd(
            "ls",
            "ls - List Directory Contents",
            "Lists files and directories in the current or specified directory.\n\
             Usage: ls [options] [path]\n\
             Flags:\n  -l  Long format with permissions, size, and timestamps\n  \
             -a  Show hidden files (starting with .)\n  \
             -h  Human-readable file sizes (KB, MB, GB)\n  \
             -R  Recurse into subdirectories\n  \
             -t  Sort by modification time, newest first\n  \
             -S  Sort by file size, largest first\n\
             Examples:\n  ls -la /home        # detailed listing including hidden files\n  \
             ls -lhS             # list sorted by size, human-readable\n  \
             ls -R src/          # recursively list src/ tree",
            &["list", "directory", "files", "dir"],
        );

        self.cmd(
            "cat",
            "cat - Concatenate and Display Files",
            "Reads one or more files and writes their contents to standard output.\n\
             Usage: cat [options] <file>...\n\
             Flags:\n  -n  Number all output lines\n  \
             -b  Number non-blank lines only\n\
             When given multiple files, they are concatenated in order.\n\
             Examples:\n  cat README.md                # print file contents\n  \
             cat part1.txt part2.txt      # concatenate two files\n  \
             cat -n script.sh             # print with line numbers",
            &["read", "print", "file", "concatenate", "display"],
        );

        self.cmd(
            "cp",
            "cp - Copy Files and Directories",
            "Copies files or directories from source to destination.\n\
             Usage: cp [options] <source>... <dest>\n\
             Flags:\n  -r  Copy directories recursively\n  \
             -i  Prompt before overwriting existing files\n  \
             -v  Verbose output, show each file as it is copied\n  \
             -p  Preserve file attributes (timestamps, permissions)\n\
             Examples:\n  cp file.txt backup.txt       # copy a single file\n  \
             cp -r src/ src_backup/       # copy entire directory\n  \
             cp -iv *.log /tmp/           # copy logs interactively with verbose output",
            &["copy", "duplicate", "file", "directory"],
        );

        self.cmd(
            "mv",
            "mv - Move or Rename Files",
            "Moves files/directories to a new location, or renames them in place.\n\
             Usage: mv [options] <source>... <dest>\n\
             Flags:\n  -i  Prompt before overwriting\n  \
             -v  Verbose, show each rename/move operation\n  \
             -n  Do not overwrite existing files\n\
             Examples:\n  mv old.txt new.txt           # rename a file\n  \
             mv *.jpg photos/            # move all jpg files into photos/\n  \
             mv -iv dir1/ /opt/dir1/     # move directory with confirmation",
            &["move", "rename", "file", "directory"],
        );

        self.cmd(
            "rm",
            "rm - Remove Files and Directories",
            "Deletes files or directories. This operation is permanent—there is no trash.\n\
             Usage: rm [options] <path>...\n\
             Flags:\n  -r  Remove directories and their contents recursively\n  \
             -f  Force removal, ignore nonexistent files, never prompt\n  \
             -i  Prompt before every removal\n  \
             -v  Verbose, print each file as it is removed\n\
             Examples:\n  rm temp.txt                  # delete a single file\n  \
             rm -rf build/               # force-remove build directory\n  \
             rm -iv *.log                # interactively remove log files",
            &["remove", "delete", "file", "directory"],
        );

        self.cmd(
            "mkdir",
            "mkdir - Create Directories",
            "Creates one or more new directories.\n\
             Usage: mkdir [options] <dir>...\n\
             Flags:\n  -p  Create parent directories as needed (no error if existing)\n  \
             -v  Print a message for each directory created\n\
             Examples:\n  mkdir projects               # create a single directory\n  \
             mkdir -p a/b/c/d             # create entire path hierarchy\n  \
             mkdir -v src tests docs      # create multiple directories",
            &["create", "directory", "folder", "make"],
        );

        self.cmd(
            "rmdir",
            "rmdir - Remove Empty Directories",
            "Removes directories only if they are empty. Use rm -r for non-empty directories.\n\
             Usage: rmdir [options] <dir>...\n\
             Flags:\n  -p  Remove directory and its empty ancestors\n  \
             -v  Verbose output\n\
             Examples:\n  rmdir old_dir                # remove an empty directory\n  \
             rmdir -p a/b/c               # remove c, then b, then a if each is empty",
            &["remove", "directory", "empty", "folder"],
        );

        self.cmd(
            "grep",
            "grep - Search Text with Patterns",
            "Searches for lines matching a pattern in files or piped input.\n\
             Usage: grep [options] <pattern> [file]...\n\
             Flags:\n  -i  Case-insensitive matching\n  \
             -r  Recurse into directories\n  \
             -n  Print line numbers alongside matches\n  \
             -c  Print only a count of matching lines per file\n  \
             -v  Invert match, show lines that do NOT match\n  \
             -l  Print only filenames containing matches\n  \
             -E  Extended regex (egrep equivalent)\n\
             Examples:\n  grep -rn \"TODO\" src/         # find all TODOs with line numbers\n  \
             grep -i error /var/log/sys   # case-insensitive error search\n  \
             ps | grep rust               # filter process list for 'rust'",
            &["search", "pattern", "regex", "find", "text", "match"],
        );

        self.cmd(
            "find",
            "find - Search for Files in a Directory Tree",
            "Walks a directory tree and matches files by name, type, size, or age.\n\
             Usage: find <path> [options]\n\
             Flags:\n  -name <pattern>   Match filename glob\n  \
             -type f           Files only\n  \
             -type d           Directories only\n  \
             -size +10M        Larger than 10 megabytes\n  \
             -mtime -7         Modified in the last 7 days\n\
             Examples:\n  find . -name \"*.rs\"          # all Rust source files under current dir\n  \
             find /tmp -type f -size +1M  # files larger than 1MB in /tmp\n  \
             find src/ -name \"*.bak\"      # find backup files in src/",
            &["search", "file", "directory", "locate", "glob"],
        );

        self.cmd(
            "wc",
            "wc - Word, Line, and Byte Count",
            "Counts lines, words, and bytes in files or standard input.\n\
             Usage: wc [options] [file]...\n\
             Flags:\n  -l  Print line count only\n  \
             -w  Print word count only\n  \
             -c  Print byte count only\n  \
             -m  Print character count only\n\
             Examples:\n  wc -l src/*.rs               # count lines in all Rust files\n  \
             cat file.txt | wc -w         # count words from piped input\n  \
             wc README.md                 # full count: lines, words, bytes",
            &["count", "lines", "words", "bytes", "statistics"],
        );

        self.cmd(
            "head",
            "head - Display Beginning of a File",
            "Prints the first N lines of a file (default 10).\n\
             Usage: head [options] <file>\n\
             Flags:\n  -n <N>  Print the first N lines (default 10)\n  \
             -c <N>  Print the first N bytes\n\
             Examples:\n  head config.toml             # first 10 lines\n  \
             head -n 20 log.txt           # first 20 lines\n  \
             head -c 256 binary.dat       # first 256 bytes",
            &["beginning", "top", "first", "lines", "preview"],
        );

        self.cmd(
            "tail",
            "tail - Display End of a File",
            "Prints the last N lines of a file (default 10). With -f, follows new output in real time.\n\
             Usage: tail [options] <file>\n\
             Flags:\n  -n <N>  Print the last N lines (default 10)\n  \
             -f      Follow the file as it grows (live tail)\n  \
             -c <N>  Print the last N bytes\n\
             Examples:\n  tail /var/log/syslog         # last 10 lines of syslog\n  \
             tail -f app.log              # follow log output live\n  \
             tail -n 50 errors.log        # last 50 lines",
            &["end", "bottom", "last", "lines", "follow", "log"],
        );

        self.cmd(
            "ps",
            "ps - List Running Processes",
            "Displays a snapshot of currently running processes.\n\
             Usage: ps [options]\n\
             Flags:\n  -a  Show processes from all users\n  \
             -u  Display user/owner column\n  \
             -x  Include processes without a controlling terminal\n\
             The output includes PID, name, CPU%, and memory usage.\n\
             Examples:\n  ps                           # list your own processes\n  \
             ps -aux                      # all processes, detailed view\n  \
             ps | grep python             # find python processes",
            &["process", "running", "pid", "status", "list"],
        );

        self.cmd(
            "kill",
            "kill - Send Signals to Processes",
            "Sends a signal to a process by PID. Default signal is SIGTERM (graceful shutdown).\n\
             Usage: kill [signal] <pid>\n\
             Common signals:\n  SIGTERM (15)  Graceful termination (default)\n  \
             SIGKILL (9)   Force kill, cannot be caught\n  \
             SIGHUP (1)    Hangup, often triggers config reload\n  \
             SIGINT (2)    Interrupt, same as Ctrl+C\n\
             Examples:\n  kill 1234                    # gracefully terminate PID 1234\n  \
             kill -9 1234                 # force kill PID 1234\n  \
             kill -HUP 5678               # send reload signal",
            &["signal", "terminate", "stop", "process", "pid"],
        );

        self.cmd(
            "top",
            "top - Real-Time System Monitor",
            "Displays a live, updating view of system resource usage and running processes.\n\
             Usage: top\n\
             Shows CPU usage, memory usage, process count, and per-process stats.\n\
             Interactive keys:\n  q  Quit\n  \
             k  Kill a process by PID\n  \
             s  Change refresh interval\n\
             Examples:\n  top                          # launch interactive system monitor\n\
             Use this to diagnose high CPU or memory usage.",
            &["monitor", "system", "cpu", "memory", "processes", "resource"],
        );

        self.cmd(
            "echo",
            "echo - Print Text to Standard Output",
            "Writes its arguments to stdout, separated by spaces, followed by a newline.\n\
             Usage: echo [options] [text]...\n\
             Flags:\n  -n  Do not append a trailing newline\n  \
             -e  Enable interpretation of backslash escapes (\\n, \\t, etc.)\n\
             Supports variable expansion: echo $HOME, echo $PATH.\n\
             Examples:\n  echo Hello, world!           # simple output\n  \
             echo -n \"no newline\"         # print without trailing newline\n  \
             echo \"Home is $HOME\"         # expand environment variable",
            &["print", "output", "text", "write", "display"],
        );

        self.cmd(
            "env",
            "env - Display or Modify Environment Variables",
            "Prints all environment variables, or runs a command with modified variables.\n\
             Usage: env [name=value]... [command]\n\
             With no arguments, prints every variable in NAME=VALUE format.\n\
             Examples:\n  env                          # list all environment variables\n  \
             env PATH=/usr/bin ls         # run ls with a custom PATH\n  \
             env | grep LANG              # find locale-related variables",
            &["environment", "variables", "path", "config", "settings"],
        );

        self.cmd(
            "pwd",
            "pwd - Print Working Directory",
            "Prints the absolute path of the current working directory.\n\
             Usage: pwd\n\
             Takes no flags. Useful in scripts to capture the current location.\n\
             Examples:\n  pwd                          # output like /home/user/projects\n  \
             CUR=$(pwd) && echo $CUR     # store current directory in a variable",
            &["directory", "current", "path", "location", "where"],
        );

        self.cmd(
            "chmod",
            "chmod - Change File Permissions",
            "Changes the access permissions of files and directories.\n\
             Usage: chmod [options] <mode> <file>...\n\
             Modes can be symbolic (u+x, go-w) or octal (755, 644).\n\
             Flags:\n  -R  Apply recursively to directories\n  \
             -v  Verbose, show each change\n\
             Common octal modes:\n  755  Owner rwx, group/others rx\n  \
             644  Owner rw, group/others r\n  \
             700  Owner rwx, no access for others\n\
             Examples:\n  chmod +x script.sh           # make script executable\n  \
             chmod 644 config.toml        # standard file permissions\n  \
             chmod -R 755 bin/            # recursively set directory permissions",
            &["permissions", "access", "mode", "executable", "read", "write"],
        );

        self.cmd(
            "df",
            "df - Disk Free Space",
            "Reports filesystem disk space usage for all mounted filesystems.\n\
             Usage: df [options] [path]\n\
             Flags:\n  -h  Human-readable sizes (KB, MB, GB)\n  \
             -T  Show filesystem type column\n  \
             -i  Show inode usage instead of block usage\n\
             Examples:\n  df -h                        # all filesystems, human sizes\n  \
             df -h /home                  # usage for filesystem containing /home\n  \
             df -Ti                       # inodes and filesystem types",
            &["disk", "space", "usage", "filesystem", "storage", "free"],
        );

        self.cmd(
            "du",
            "du - Disk Usage by File/Directory",
            "Estimates and reports the disk space used by files and directories.\n\
             Usage: du [options] [path]...\n\
             Flags:\n  -h  Human-readable sizes\n  \
             -s  Summary—show only the total for each argument\n  \
             -d <N>  Limit directory depth to N levels\n  \
             --max-depth <N>  Same as -d\n\
             Examples:\n  du -sh *                     # summary size of each item in cwd\n  \
             du -h -d 1 /var              # one level deep under /var\n  \
             du -sh ~/projects            # total size of projects directory",
            &["disk", "usage", "size", "space", "directory"],
        );

        self.cmd(
            "date",
            "date - Display or Set the System Date and Time",
            "Prints the current date and time, optionally in a custom format.\n\
             Usage: date [options] [+format]\n\
             Flags:\n  -u  Display UTC instead of local time\n  \
             -I  ISO 8601 format output\n\
             Format codes: %Y year, %m month, %d day, %H hour, %M minute, %S second.\n\
             Examples:\n  date                         # e.g. Wed Mar 11 14:30:00 PST 2026\n  \
             date +\"%Y-%m-%d %H:%M:%S\"   # 2026-03-11 14:30:00\n  \
             date -u                      # current UTC time",
            &["time", "clock", "timestamp", "calendar", "now"],
        );

        self.cmd(
            "uptime",
            "uptime - System Uptime and Load",
            "Shows how long the system has been running, the number of users, and load averages.\n\
             Usage: uptime\n\
             Output includes: current time, uptime duration, user count, and 1/5/15-minute load.\n\
             Examples:\n  uptime                       # e.g. up 3 days, 2:15, load: 0.5, 0.3, 0.2\n\
             Use this to quickly check system health and load trends.",
            &["system", "load", "running", "health", "time"],
        );

        self.cmd(
            "whoami",
            "whoami - Display Current Username",
            "Prints the username associated with the current effective user ID.\n\
             Usage: whoami\n\
             Takes no flags. Useful in scripts to branch on the running user.\n\
             Examples:\n  whoami                       # e.g. root, admin, user\n  \
             if [ $(whoami) = root ]; then echo \"Running as root\"; fi",
            &["user", "username", "identity", "current"],
        );

        self.cmd(
            "hostname",
            "hostname - Display System Hostname",
            "Prints the network hostname of the machine.\n\
             Usage: hostname\n\
             The hostname identifies this machine on the network.\n\
             Examples:\n  hostname                     # e.g. aios-workstation\n\
             Use this in scripts or prompts to identify which system you are on.",
            &["host", "name", "machine", "network", "system"],
        );

        self.cmd(
            "cd",
            "cd - Change Directory",
            "Changes the current working directory to the specified path.\n\
             Usage: cd [path]\n\
             Special paths:\n  cd ~       Home directory\n  \
             cd ..     Parent directory\n  \
             cd -      Previous directory\n  \
             cd /      Root directory\n\
             With no argument, cd returns to the home directory.\n\
             Examples:\n  cd /var/log                  # absolute path\n  \
             cd ../sibling                # relative path up then down\n  \
             cd -                         # toggle between last two directories",
            &["directory", "navigate", "change", "path", "chdir"],
        );

        self.cmd(
            "export",
            "export - Set Environment Variables",
            "Sets or exports environment variables so they are visible to child processes.\n\
             Usage: export NAME=VALUE\n\
             Without a value, export NAME makes an existing shell variable available to children.\n\
             Variables persist for the duration of the session unless saved to a profile file.\n\
             Examples:\n  export PATH=$PATH:/opt/bin   # append to PATH\n  \
             export EDITOR=vim            # set default editor\n  \
             export RUST_LOG=debug        # enable debug logging for Rust apps",
            &["environment", "variable", "set", "path", "config"],
        );
    }

    fn add_concepts(&mut self) {
        self.concept(
            "aios-overview",
            "AIOS Overview",
            "AIOS (AI Operating System) is an AI-native operating system written entirely in Rust. \
             It combines a Unix-like userland with deep LLM integration, giving every command, \
             pipe, and shell session first-class access to AI capabilities.\n\n\
             Architecture layers:\n  \
             1. aios-kernel — Core kernel with VFS, process scheduler, memory management, and IPC\n  \
             2. aios-shell — Interactive shell supporting natural language, @ AI prefix, and AI pipes\n  \
             3. aios-core — Shared types, configuration, and cross-crate utilities\n  \
             4. aios-llm — Pluggable LLM backend (local, network, or cloud)\n  \
             5. aios-knowledge — Embedded knowledge base with search (this crate)\n  \
             6. aios-commands — Built-in command implementations\n  \
             7. aios-ai — AI agent, prompt construction, and tool-use orchestration\n\n\
             AIOS boots into a fully interactive shell where users can mix traditional commands \
             with natural language queries. The OS is self-aware: it can introspect its own \
             documentation, suggest commands, explain errors, and learn from user patterns.",
            &["architecture", "overview", "system", "rust", "ai", "operating system"],
        );

        self.concept(
            "llm-config",
            "LLM Configuration",
            "AIOS supports three LLM backends, configured in the system config file or via \
             environment variables.\n\n\
             1. Local — Runs a quantized model on-device via a built-in inference runtime. \
             Best for privacy and offline usage. Set LLM_BACKEND=local and LLM_MODEL_PATH to \
             the model file.\n\n\
             2. Network — Connects to a LLM server on the local network (e.g., Ollama, vLLM). \
             Set LLM_BACKEND=network and LLM_ENDPOINT to the server URL.\n\n\
             3. Cloud — Uses a cloud API (OpenAI-compatible). Set LLM_BACKEND=cloud, \
             LLM_ENDPOINT to the API base URL, and LLM_API_KEY to your key.\n\n\
             The active backend can be queried with `llm status` and switched at runtime with \
             `llm use <backend>`. Temperature, max tokens, and system prompt are adjustable via \
             `llm config set <key> <value>`.",
            &["llm", "model", "config", "local", "cloud", "network", "ai", "inference"],
        );

        self.concept(
            "ai-shell",
            "AI Shell Usage",
            "The AIOS shell extends a traditional Unix shell with AI capabilities.\n\n\
             @ Prefix — Typing @ followed by a natural language request invokes the AI agent. \
             Example: `@ list all rust files larger than 1MB` translates to an appropriate \
             command and executes it after confirmation.\n\n\
             AI Pipes — The special `|ai` pipe sends command output to the LLM for \
             summarization, transformation, or analysis. Example: `ps -aux |ai summarize \
             top CPU consumers`.\n\n\
             Conversational Mode — Press Ctrl+A to toggle conversational mode, where every \
             line is treated as a natural language prompt instead of a shell command.\n\n\
             AI Explain — Append `--explain` to any command to get a plain-English explanation \
             of what it does before running it.\n\n\
             The AI has access to the knowledge base, so it can answer questions about AIOS \
             itself, Unix concepts, and the user's environment.",
            &["shell", "ai", "natural language", "prefix", "pipe", "agent", "explain"],
        );

        self.concept(
            "file-permissions",
            "File Permissions",
            "AIOS uses the standard Unix permission model. Every file and directory has an \
             owner, a group, and permission bits for read (r), write (w), and execute (x).\n\n\
             Permission triplet: owner | group | others. Each has rwx bits.\n\
             Numeric (octal) representation: r=4, w=2, x=1. Sum for each triplet.\n  \
             755 = rwxr-xr-x (owner full, others read+execute)\n  \
             644 = rw-r--r-- (owner read+write, others read only)\n  \
             700 = rwx------ (owner only)\n\n\
             Use `chmod` to change permissions, `ls -l` to view them.\n\
             The execute bit on a directory means permission to list/enter it.\n\
             Special bits: setuid (4000), setgid (2000), sticky (1000).",
            &["permissions", "chmod", "rwx", "octal", "owner", "group", "access"],
        );

        self.concept(
            "process-management",
            "Process Management",
            "A process is a running instance of a program. AIOS manages processes through \
             its built-in scheduler.\n\n\
             Every process has:\n  - PID (process ID) — unique numeric identifier\n  \
             - Parent PID — the process that spawned it\n  \
             - State — running, sleeping, stopped, or zombie\n  \
             - Priority — scheduling weight\n\n\
             Key commands:\n  ps          List running processes\n  \
             top         Live resource monitor\n  \
             kill <pid>  Send a signal (default SIGTERM)\n  \
             kill -9     Force kill (SIGKILL)\n\n\
             Signals are the inter-process communication primitive for control:\n  \
             SIGTERM (15) — polite request to exit\n  \
             SIGKILL (9)  — immediate forced termination\n  \
             SIGINT (2)   — interrupt from keyboard (Ctrl+C)\n  \
             SIGSTOP      — pause the process\n  \
             SIGCONT      — resume a paused process",
            &["process", "pid", "signal", "kill", "scheduler", "sigterm", "sigkill"],
        );

        self.concept(
            "env-vars",
            "Environment Variables",
            "Environment variables are key-value pairs inherited by child processes. They \
             configure system behavior without hardcoding values.\n\n\
             Common variables:\n  PATH     — Directories searched for executables, colon-separated\n  \
             HOME     — Current user's home directory\n  \
             EDITOR   — Default text editor\n  \
             LANG     — Locale setting\n  \
             RUST_LOG — Logging verbosity for Rust applications\n  \
             LLM_BACKEND — Active AI backend (local/network/cloud)\n\n\
             Set a variable: export NAME=value\n\
             View all variables: env\n\
             View one variable: echo $NAME\n\
             Unset a variable: unset NAME\n\n\
             Variables set with export persist for the current session. To make them permanent, \
             add the export line to your shell profile (~/.aiosrc or ~/.profile).",
            &["environment", "variable", "path", "export", "config", "settings"],
        );

        self.concept(
            "pipes-redirects",
            "Pipes and Redirects",
            "Pipes and redirects connect commands together and control where data flows.\n\n\
             Pipe ( | ) — Sends stdout of one command into stdin of the next.\n  \
             Example: ls -la | grep \".rs\"   →   list files, filter for Rust sources\n\n\
             Output redirect ( > ) — Write stdout to a file, replacing its contents.\n  \
             Example: echo hello > greet.txt\n\n\
             Append redirect ( >> ) — Append stdout to a file.\n  \
             Example: echo more >> greet.txt\n\n\
             Input redirect ( < ) — Read stdin from a file.\n  \
             Example: wc -l < data.csv\n\n\
             Stderr redirect ( 2> ) — Redirect error output.\n  \
             Example: cmd 2> errors.log\n\n\
             Combine: cmd > out.log 2>&1  — merge stderr into stdout file.\n\n\
             AI pipe ( |ai ) — AIOS extension that sends output to the LLM for processing.\n  \
             Example: cat log.txt |ai summarize the errors",
            &["pipe", "redirect", "stdin", "stdout", "stderr", "stream", "output", "input"],
        );

        self.concept(
            "shell-scripting",
            "Shell Scripting",
            "AIOS supports shell scripts in .aios files. Scripts are sequences of commands \
             executed in order.\n\n\
             Basic structure:\n  #!/usr/bin/env aios-shell\n  \
             # This is a comment\n  \
             echo \"Starting build...\"\n  \
             mkdir -p build\n  \
             cp src/*.rs build/\n  \
             echo \"Done.\"\n\n\
             Variables: name=\"world\" then echo \"Hello $name\"\n\
             Conditionals: if [ -f file.txt ]; then echo exists; fi\n\
             Loops: for f in *.txt; do echo $f; done\n\
             Exit codes: every command returns 0 for success, non-zero for failure.\n  \
             Use && to chain (run next only if previous succeeded).\n  \
             Use || for fallback (run next only if previous failed).\n\n\
             AI integration in scripts: use `@ <prompt>` lines to invoke the AI agent \
             inline. The agent's output becomes the command that runs next.",
            &["script", "automation", "batch", "aios", "shell", "programming", "loop", "if"],
        );
    }
}

impl Default for KnowledgeIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_doc_count() {
        let idx = KnowledgeIndex::new();
        assert!(idx.store().document_count() >= 35);
    }

    #[test]
    fn test_query_finds_command() {
        let idx = KnowledgeIndex::new();
        let results = idx.query("list files in a directory", 3);
        assert!(!results.is_empty());
        let ids: Vec<&str> = results.iter().map(|r| r.document.id.as_str()).collect();
        assert!(ids.contains(&"cmd-ls"), "expected cmd-ls in results: {:?}", ids);
    }

    #[test]
    fn test_query_for_context_format() {
        let idx = KnowledgeIndex::new();
        let ctx = idx.query_for_context("how do pipes work");
        assert!(ctx.contains("## Relevant Knowledge"));
        assert!(ctx.contains("###"));
    }
}
