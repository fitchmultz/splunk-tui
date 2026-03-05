//! Shell completion generation command.
//!
//! Responsibilities:
//! - Generate shell completion scripts for various shells (bash, zsh, fish, powershell, elvish).
//! - Generate dynamic completion helper functions when --dynamic flag is used.
//!
//! Does NOT handle:
//! - Direct installation of completions (user must redirect output to appropriate location).
//! - Fetching dynamic values (handled by dynamic_complete module and Complete command).

use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{Shell, generate};
use std::io;

/// Generate shell completions for the specified shell.
///
/// # Arguments
/// * `shell` - The target shell for completion generation
/// * `dynamic` - Whether to include dynamic completion support
/// * `cache_ttl` - Cache TTL in seconds for dynamic completions
///
/// # Returns
/// Result indicating success or failure of the operation
pub fn run(shell: Shell, dynamic: bool, cache_ttl: u64) -> Result<()> {
    let mut cmd = crate::args::Cli::command();

    // Generate static completions
    generate(shell, &mut cmd, "splunk-cli", &mut io::stdout());

    // Generate dynamic completion helpers if requested
    if dynamic {
        println!();
        print_dynamic_helpers(shell, cache_ttl)?;
    }

    Ok(())
}

/// Print shell-specific dynamic completion helper functions.
fn print_dynamic_helpers(shell: Shell, cache_ttl: u64) -> Result<()> {
    match shell {
        Shell::Bash => print_bash_helpers(cache_ttl),
        Shell::Zsh => print_zsh_helpers(cache_ttl),
        Shell::Fish => print_fish_helpers(cache_ttl),
        Shell::PowerShell => print_pwsh_helpers(cache_ttl),
        Shell::Elvish => print_elvish_helpers(cache_ttl),
        _ => {
            // For unsupported shells, just skip dynamic helpers
            Ok(())
        }
    }
}

fn print_bash_helpers(cache_ttl: u64) -> Result<()> {
    let script = format!(
        r#"
# Dynamic completion helpers for splunk-cli
# Cache TTL: {ttl} seconds

# Helper to fetch profile names from config (offline, no server needed)
_splunk_cli_complete_profiles() {{
    local profiles
    if command -v splunk-cli &> /dev/null; then
        profiles=$(splunk-cli complete profiles 2>/dev/null)
        if [[ -n "$profiles" ]]; then
            echo "$profiles"
        fi
    fi
}}

# Helper to fetch index names from server
_splunk_cli_complete_indexes() {{
    local indexes
    if command -v splunk-cli &> /dev/null; then
        indexes=$(splunk-cli complete indexes 2>/dev/null)
        if [[ -n "$indexes" ]]; then
            echo "$indexes"
        fi
    fi
}}

# Helper to fetch saved search names from server
_splunk_cli_complete_saved_searches() {{
    local searches
    if command -v splunk-cli &> /dev/null; then
        searches=$(splunk-cli complete saved-searches 2>/dev/null)
        if [[ -n "$searches" ]]; then
            echo "$searches"
        fi
    fi
}}

# Helper to fetch job SIDs from server
_splunk_cli_complete_jobs() {{
    local jobs
    if command -v splunk-cli &> /dev/null; then
        jobs=$(splunk-cli complete jobs 2>/dev/null)
        if [[ -n "$jobs" ]]; then
            echo "$jobs"
        fi
    fi
}}

# Helper to fetch app names from server
_splunk_cli_complete_apps() {{
    local apps
    if command -v splunk-cli &> /dev/null; then
        apps=$(splunk-cli complete apps 2>/dev/null)
        if [[ -n "$apps" ]]; then
            echo "$apps"
        fi
    fi
}}
"#,
        ttl = cache_ttl
    );

    println!("{}", script);
    Ok(())
}

fn print_zsh_helpers(cache_ttl: u64) -> Result<()> {
    let script = format!(
        r#"
# Dynamic completion helpers for splunk-cli
# Cache TTL: {ttl} seconds

# Helper function to fetch profile names
_splunk_cli_profiles() {{
    local -a profiles
    if (( $+commands[splunk-cli] )); then
        profiles=(${{(f)"$(splunk-cli complete profiles 2>/dev/null)"}})
    fi
    echo "${{profiles[@]}}"
}}

# Helper function to fetch index names
_splunk_cli_indexes() {{
    local -a indexes
    if (( $+commands[splunk-cli] )); then
        indexes=(${{(f)"$(splunk-cli complete indexes 2>/dev/null)"}})
    fi
    echo "${{indexes[@]}}"
}}

# Helper function to fetch saved search names
_splunk_cli_saved_searches() {{
    local -a searches
    if (( $+commands[splunk-cli] )); then
        searches=(${{(f)"$(splunk-cli complete saved-searches 2>/dev/null)"}})
    fi
    echo "${{searches[@]}}"
}}

# Helper function to fetch job SIDs
_splunk_cli_jobs() {{
    local -a jobs
    if (( $+commands[splunk-cli] )); then
        jobs=(${{(f)"$(splunk-cli complete jobs 2>/dev/null)"}})
    fi
    echo "${{jobs[@]}}"
}}

# Helper function to fetch app names
_splunk_cli_apps() {{
    local -a apps
    if (( $+commands[splunk-cli] )); then
        apps=(${{(f)"$(splunk-cli complete apps 2>/dev/null)"}})
    fi
    echo "${{apps[@]}}"
}}
"#,
        ttl = cache_ttl
    );

    println!("{}", script);
    Ok(())
}

fn print_fish_helpers(_cache_ttl: u64) -> Result<()> {
    let script = r#"
# Dynamic completion helpers for splunk-cli

function __splunk_cli_profiles
    if command -q splunk-cli
        splunk-cli complete profiles 2>/dev/null
    end
end

function __splunk_cli_indexes
    if command -q splunk-cli
        splunk-cli complete indexes 2>/dev/null
    end
end

function __splunk_cli_saved_searches
    if command -q splunk-cli
        splunk-cli complete saved-searches 2>/dev/null
    end
end

function __splunk_cli_jobs
    if command -q splunk-cli
        splunk-cli complete jobs 2>/dev/null
    end
end

function __splunk_cli_apps
    if command -q splunk-cli
        splunk-cli complete apps 2>/dev/null
    end
end
"#;

    println!("{}", script);
    Ok(())
}

fn print_pwsh_helpers(_cache_ttl: u64) -> Result<()> {
    let script = r#"
# Dynamic completion helpers for splunk-cli

function Get-SplunkCliProfiles {
    if (Get-Command splunk-cli -ErrorAction SilentlyContinue) {
        splunk-cli complete profiles 2>$null
    }
}

function Get-SplunkCliIndexes {
    if (Get-Command splunk-cli -ErrorAction SilentlyContinue) {
        splunk-cli complete indexes 2>$null
    }
}

function Get-SplunkCliSavedSearches {
    if (Get-Command splunk-cli -ErrorAction SilentlyContinue) {
        splunk-cli complete saved-searches 2>$null
    }
}

function Get-SplunkCliJobs {
    if (Get-Command splunk-cli -ErrorAction SilentlyContinue) {
        splunk-cli complete jobs 2>$null
    }
}

function Get-SplunkCliApps {
    if (Get-Command splunk-cli -ErrorAction SilentlyContinue) {
        splunk-cli complete apps 2>$null
    }
}
"#;

    println!("{}", script);
    Ok(())
}

fn print_elvish_helpers(_cache_ttl: u64) -> Result<()> {
    // Elvish has its own structured completion system
    // Dynamic completions would need to be implemented via its module system
    // For now, provide basic documentation
    let script = r#"
# Dynamic completion helpers for splunk-cli
# Note: Elvish dynamic completions require custom module implementation
# The following commands can be used:
#   splunk-cli complete profiles
#   splunk-cli complete indexes
#   splunk-cli complete saved-searches
#   splunk-cli complete jobs
#   splunk-cli complete apps
"#;

    println!("{}", script);
    Ok(())
}
