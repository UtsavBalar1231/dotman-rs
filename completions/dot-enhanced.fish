# Enhanced Fish completion for dotman (dot)
# Provides dynamic completions for branches, remotes, commits, config keys, and files

# Helper function to get branches
function __dot_get_branches
    if test -d "$HOME/.dotman"
        dot branch list 2>/dev/null | grep '^\*\|^  ' | sed 's/^[* ] //' | grep -v '^$'
    end
end

# Helper function to get remotes
function __dot_get_remotes
    if test -d "$HOME/.dotman"
        dot remote list 2>/dev/null | grep -v '^$'
    end
end

# Helper function to get recent commits (last 20)
function __dot_get_commits
    if test -d "$HOME/.dotman"
        # Get commit IDs and their first line of message
        dot log --oneline -n 20 2>/dev/null | awk '{print $1}'
    end
end

# Helper function to get untracked files
function __dot_get_untracked_files
    if test -d "$HOME/.dotman"
        dot status --untracked 2>/dev/null | grep '^?' | awk '{print $2}'
    end
end

# Helper function to get tracked files
function __dot_get_tracked_files
    if test -d "$HOME/.dotman"
        dot status 2>/dev/null | grep -E '^(M|A|D)' | awk '{print $2}'
    end
end

# Helper function to get current branch
function __dot_get_current_branch
    if test -d "$HOME/.dotman"
        dot branch list 2>/dev/null | grep '^\*' | sed 's/^[* ] //'
    end
end

# Helper function to check if we need a subcommand
function __dot_needs_command
    set -l cmd (commandline -opc)
    test (count $cmd) -eq 1
end

# Helper function to get the current command
function __dot_get_command
    set -l cmd (commandline -opc)
    if test (count $cmd) -ge 2
        echo $cmd[2]
    end
end

# Helper function to get the current subcommand
function __dot_get_subcommand
    set -l cmd (commandline -opc)
    if test (count $cmd) -ge 3
        echo $cmd[3]
    end
end

# Disable file completion by default
complete -c dot -f

# Main commands
complete -c dot -n __dot_needs_command -a add -d 'Add files to be tracked'
complete -c dot -n __dot_needs_command -a status -d 'Show the working tree status'
complete -c dot -n __dot_needs_command -a commit -d 'Record changes to the repository'
complete -c dot -n __dot_needs_command -a checkout -d 'Switch branches or restore working tree files'
complete -c dot -n __dot_needs_command -a reset -d 'Reset current HEAD to the specified state'
complete -c dot -n __dot_needs_command -a push -d 'Update remote refs along with associated objects'
complete -c dot -n __dot_needs_command -a pull -d 'Fetch from and integrate with another repository'
complete -c dot -n __dot_needs_command -a init -d 'Initialize a new dotman repository'
complete -c dot -n __dot_needs_command -a show -d 'Show various types of objects'
complete -c dot -n __dot_needs_command -a log -d 'Show commit logs'
complete -c dot -n __dot_needs_command -a diff -d 'Show changes between commits'
complete -c dot -n __dot_needs_command -a rm -d 'Remove files from tracking'
complete -c dot -n __dot_needs_command -a remote -d 'Manage remote repositories'
complete -c dot -n __dot_needs_command -a branch -d 'Manage branches'
complete -c dot -n __dot_needs_command -a config -d 'Get and set repository or user options'
complete -c dot -n __dot_needs_command -a completion -d 'Generate shell completion scripts'
complete -c dot -n __dot_needs_command -a help -d 'Show help for a command'

# Add command
complete -c dot -n "__fish_seen_subcommand_from add" -s f -l force -d 'Force adding files'
complete -c dot -n "__fish_seen_subcommand_from add" -s h -l help -d 'Show help'
complete -c dot -n "__fish_seen_subcommand_from add" -a "(__dot_get_untracked_files)" -d 'Untracked file'
complete -c dot -n "__fish_seen_subcommand_from add" -F  # Also allow file completion

# Status command
complete -c dot -n "__fish_seen_subcommand_from status" -s s -l short -d 'Show short format'
complete -c dot -n "__fish_seen_subcommand_from status" -s u -l untracked -d 'Show untracked files'
complete -c dot -n "__fish_seen_subcommand_from status" -s h -l help -d 'Show help'

# Commit command
complete -c dot -n "__fish_seen_subcommand_from commit" -s m -l message -r -d 'Commit message'
complete -c dot -n "__fish_seen_subcommand_from commit" -s a -l all -d 'Commit all tracked files'
complete -c dot -n "__fish_seen_subcommand_from commit" -l amend -d 'Amend the last commit'
complete -c dot -n "__fish_seen_subcommand_from commit" -s h -l help -d 'Show help'

# Checkout command
complete -c dot -n "__fish_seen_subcommand_from checkout" -s f -l force -d 'Force checkout'
complete -c dot -n "__fish_seen_subcommand_from checkout" -s h -l help -d 'Show help'
complete -c dot -n "__fish_seen_subcommand_from checkout" -a "(__dot_get_branches)" -d 'Branch'
complete -c dot -n "__fish_seen_subcommand_from checkout" -a "(__dot_get_commits)" -d 'Commit'
complete -c dot -n "__fish_seen_subcommand_from checkout" -a HEAD -d 'HEAD'

# Reset command
complete -c dot -n "__fish_seen_subcommand_from reset" -l hard -d 'Reset working tree and index'
complete -c dot -n "__fish_seen_subcommand_from reset" -l soft -d 'Reset only HEAD'
complete -c dot -n "__fish_seen_subcommand_from reset" -s h -l help -d 'Show help'
complete -c dot -n "__fish_seen_subcommand_from reset" -a "(__dot_get_commits)" -d 'Commit'
complete -c dot -n "__fish_seen_subcommand_from reset" -a "(__dot_get_branches)" -d 'Branch'
complete -c dot -n "__fish_seen_subcommand_from reset" -a HEAD -d 'HEAD'

# Push command
complete -c dot -n "__fish_seen_subcommand_from push" -s h -l help -d 'Show help'
complete -c dot -n "__fish_seen_subcommand_from push; and not __fish_seen_argument (__dot_get_remotes)" -a "(__dot_get_remotes)" -d 'Remote'
complete -c dot -n "__fish_seen_subcommand_from push; and __fish_seen_argument (__dot_get_remotes)" -a "(__dot_get_branches)" -d 'Branch'

# Pull command
complete -c dot -n "__fish_seen_subcommand_from pull" -s h -l help -d 'Show help'
complete -c dot -n "__fish_seen_subcommand_from pull; and not __fish_seen_argument (__dot_get_remotes)" -a "(__dot_get_remotes)" -d 'Remote'
complete -c dot -n "__fish_seen_subcommand_from pull; and __fish_seen_argument (__dot_get_remotes)" -a "(__dot_get_branches)" -d 'Branch'

# Init command
complete -c dot -n "__fish_seen_subcommand_from init" -s b -l bare -d 'Create a bare repository'
complete -c dot -n "__fish_seen_subcommand_from init" -s h -l help -d 'Show help'

# Show command
complete -c dot -n "__fish_seen_subcommand_from show" -s h -l help -d 'Show help'
complete -c dot -n "__fish_seen_subcommand_from show" -a "(__dot_get_commits)" -d 'Commit'
complete -c dot -n "__fish_seen_subcommand_from show" -a "(__dot_get_branches)" -d 'Branch'
complete -c dot -n "__fish_seen_subcommand_from show" -a HEAD -d 'HEAD'

# Log command
complete -c dot -n "__fish_seen_subcommand_from log" -s n -l limit -r -d 'Number of commits to show'
complete -c dot -n "__fish_seen_subcommand_from log" -l oneline -d 'Show in oneline format'
complete -c dot -n "__fish_seen_subcommand_from log" -s h -l help -d 'Show help'
complete -c dot -n "__fish_seen_subcommand_from log" -a "(__dot_get_branches)" -d 'Branch'
complete -c dot -n "__fish_seen_subcommand_from log" -a "(__dot_get_commits)" -d 'Commit'
complete -c dot -n "__fish_seen_subcommand_from log" -a HEAD -d 'HEAD'

# Diff command
complete -c dot -n "__fish_seen_subcommand_from diff" -s h -l help -d 'Show help'
complete -c dot -n "__fish_seen_subcommand_from diff" -a "(__dot_get_commits)" -d 'Commit'
complete -c dot -n "__fish_seen_subcommand_from diff" -a "(__dot_get_branches)" -d 'Branch'
complete -c dot -n "__fish_seen_subcommand_from diff" -a HEAD -d 'HEAD'

# Rm command
complete -c dot -n "__fish_seen_subcommand_from rm" -s c -l cached -d 'Remove from index only'
complete -c dot -n "__fish_seen_subcommand_from rm" -s f -l force -d 'Force removal'
complete -c dot -n "__fish_seen_subcommand_from rm" -s i -l interactive -d 'Interactive removal'
complete -c dot -n "__fish_seen_subcommand_from rm" -s h -l help -d 'Show help'
complete -c dot -n "__fish_seen_subcommand_from rm" -F  # Allow file completion

# Remote command
complete -c dot -n "__fish_seen_subcommand_from remote; and not __fish_seen_subcommand_from list add remove set-url show rename" -a list -d 'List all remotes'
complete -c dot -n "__fish_seen_subcommand_from remote; and not __fish_seen_subcommand_from list add remove set-url show rename" -a add -d 'Add a new remote'
complete -c dot -n "__fish_seen_subcommand_from remote; and not __fish_seen_subcommand_from list add remove set-url show rename" -a remove -d 'Remove a remote'
complete -c dot -n "__fish_seen_subcommand_from remote; and not __fish_seen_subcommand_from list add remove set-url show rename" -a set-url -d 'Set the URL for a remote'
complete -c dot -n "__fish_seen_subcommand_from remote; and not __fish_seen_subcommand_from list add remove set-url show rename" -a show -d 'Show information about a remote'
complete -c dot -n "__fish_seen_subcommand_from remote; and not __fish_seen_subcommand_from list add remove set-url show rename" -a rename -d 'Rename a remote'

# Remote subcommands
complete -c dot -n "__fish_seen_subcommand_from remote; and __fish_seen_subcommand_from list" -s h -l help -d 'Show help'
complete -c dot -n "__fish_seen_subcommand_from remote; and __fish_seen_subcommand_from add" -s h -l help -d 'Show help'
complete -c dot -n "__fish_seen_subcommand_from remote; and __fish_seen_subcommand_from remove" -s h -l help -d 'Show help'
complete -c dot -n "__fish_seen_subcommand_from remote; and __fish_seen_subcommand_from remove" -a "(__dot_get_remotes)" -d 'Remote'
complete -c dot -n "__fish_seen_subcommand_from remote; and __fish_seen_subcommand_from set-url" -s h -l help -d 'Show help'
complete -c dot -n "__fish_seen_subcommand_from remote; and __fish_seen_subcommand_from set-url" -a "(__dot_get_remotes)" -d 'Remote'
complete -c dot -n "__fish_seen_subcommand_from remote; and __fish_seen_subcommand_from show" -s h -l help -d 'Show help'
complete -c dot -n "__fish_seen_subcommand_from remote; and __fish_seen_subcommand_from show" -a "(__dot_get_remotes)" -d 'Remote'
complete -c dot -n "__fish_seen_subcommand_from remote; and __fish_seen_subcommand_from rename" -s h -l help -d 'Show help'
complete -c dot -n "__fish_seen_subcommand_from remote; and __fish_seen_subcommand_from rename" -a "(__dot_get_remotes)" -d 'Remote'

# Branch command
complete -c dot -n "__fish_seen_subcommand_from branch; and not __fish_seen_subcommand_from list create delete rename set-upstream unset-upstream" -a list -d 'List all branches'
complete -c dot -n "__fish_seen_subcommand_from branch; and not __fish_seen_subcommand_from list create delete rename set-upstream unset-upstream" -a create -d 'Create a new branch'
complete -c dot -n "__fish_seen_subcommand_from branch; and not __fish_seen_subcommand_from list create delete rename set-upstream unset-upstream" -a delete -d 'Delete a branch'
complete -c dot -n "__fish_seen_subcommand_from branch; and not __fish_seen_subcommand_from list create delete rename set-upstream unset-upstream" -a rename -d 'Rename a branch'
complete -c dot -n "__fish_seen_subcommand_from branch; and not __fish_seen_subcommand_from list create delete rename set-upstream unset-upstream" -a set-upstream -d 'Set upstream tracking for a branch'
complete -c dot -n "__fish_seen_subcommand_from branch; and not __fish_seen_subcommand_from list create delete rename set-upstream unset-upstream" -a unset-upstream -d 'Remove upstream tracking for a branch'

# Branch subcommands
complete -c dot -n "__fish_seen_subcommand_from branch; and __fish_seen_subcommand_from list" -s h -l help -d 'Show help'
complete -c dot -n "__fish_seen_subcommand_from branch; and __fish_seen_subcommand_from create" -s f -l from -r -d 'Starting point'
complete -c dot -n "__fish_seen_subcommand_from branch; and __fish_seen_subcommand_from create" -s h -l help -d 'Show help'
complete -c dot -n "__fish_seen_subcommand_from branch; and __fish_seen_subcommand_from delete" -s f -l force -d 'Force deletion'
complete -c dot -n "__fish_seen_subcommand_from branch; and __fish_seen_subcommand_from delete" -s h -l help -d 'Show help'

# Delete branch completion (exclude current branch)
function __dot_get_deletable_branches
    set -l current (__dot_get_current_branch)
    if test -d "$HOME/.dotman"
        dot branch list 2>/dev/null | grep '^\*\|^  ' | sed 's/^[* ] //' | grep -v "^$current\$" | grep -v '^$'
    end
end
complete -c dot -n "__fish_seen_subcommand_from branch; and __fish_seen_subcommand_from delete" -a "(__dot_get_deletable_branches)" -d 'Branch'

complete -c dot -n "__fish_seen_subcommand_from branch; and __fish_seen_subcommand_from rename" -s h -l help -d 'Show help'
complete -c dot -n "__fish_seen_subcommand_from branch; and __fish_seen_subcommand_from rename" -a "(__dot_get_branches)" -d 'Branch'
complete -c dot -n "__fish_seen_subcommand_from branch; and __fish_seen_subcommand_from set-upstream" -s b -l branch -r -d 'Branch name'
complete -c dot -n "__fish_seen_subcommand_from branch; and __fish_seen_subcommand_from set-upstream" -l remote-branch -r -d 'Remote branch name'
complete -c dot -n "__fish_seen_subcommand_from branch; and __fish_seen_subcommand_from set-upstream" -s h -l help -d 'Show help'
complete -c dot -n "__fish_seen_subcommand_from branch; and __fish_seen_subcommand_from set-upstream" -a "(__dot_get_remotes)" -d 'Remote'
complete -c dot -n "__fish_seen_subcommand_from branch; and __fish_seen_subcommand_from unset-upstream" -s h -l help -d 'Show help'
complete -c dot -n "__fish_seen_subcommand_from branch; and __fish_seen_subcommand_from unset-upstream" -a "(__dot_get_branches)" -d 'Branch'

# Config command
complete -c dot -n "__fish_seen_subcommand_from config" -l unset -d 'Unset the configuration key'
complete -c dot -n "__fish_seen_subcommand_from config" -l list -d 'List all configuration values'
complete -c dot -n "__fish_seen_subcommand_from config" -s h -l help -d 'Show help'

# Config keys
complete -c dot -n "__fish_seen_subcommand_from config; and not __fish_seen_argument --unset" -a "user.name" -d 'Set user name for commits'
complete -c dot -n "__fish_seen_subcommand_from config; and not __fish_seen_argument --unset" -a "user.email" -d 'Set user email for commits'
complete -c dot -n "__fish_seen_subcommand_from config; and not __fish_seen_argument --unset" -a "core.compression" -d 'Enable/disable compression'
complete -c dot -n "__fish_seen_subcommand_from config; and not __fish_seen_argument --unset" -a "core.compression_level" -d 'Set compression level (1-22)'
complete -c dot -n "__fish_seen_subcommand_from config; and not __fish_seen_argument --unset" -a "core.default_branch" -d 'Set default branch name'
complete -c dot -n "__fish_seen_subcommand_from config; and not __fish_seen_argument --unset" -a "performance.parallel_threads" -d 'Number of parallel threads'
complete -c dot -n "__fish_seen_subcommand_from config; and not __fish_seen_argument --unset" -a "performance.mmap_threshold" -d 'Memory-mapped I/O threshold'
complete -c dot -n "__fish_seen_subcommand_from config; and not __fish_seen_argument --unset" -a "performance.cache_size" -d 'Cache size in MB'
complete -c dot -n "__fish_seen_subcommand_from config; and not __fish_seen_argument --unset" -a "performance.use_hard_links" -d 'Enable hard links'
complete -c dot -n "__fish_seen_subcommand_from config; and not __fish_seen_argument --unset" -a "tracking.follow_symlinks" -d 'Follow symbolic links'
complete -c dot -n "__fish_seen_subcommand_from config; and not __fish_seen_argument --unset" -a "tracking.preserve_permissions" -d 'Preserve file permissions'

# Completion command
complete -c dot -n "__fish_seen_subcommand_from completion" -s h -l help -d 'Show help'
complete -c dot -n "__fish_seen_subcommand_from completion" -a bash -d 'Bash shell'
complete -c dot -n "__fish_seen_subcommand_from completion" -a zsh -d 'Zsh shell'
complete -c dot -n "__fish_seen_subcommand_from completion" -a fish -d 'Fish shell'
complete -c dot -n "__fish_seen_subcommand_from completion" -a powershell -d 'PowerShell'
complete -c dot -n "__fish_seen_subcommand_from completion" -a elvish -d 'Elvish shell'

# Also preserve the basic completion as a fallback
if test -f (dirname (status -f))/dot.fish
    # Source the basic completion but don't let it override our enhanced one
    source (dirname (status -f))/dot.fish 2>/dev/null; or true
end
