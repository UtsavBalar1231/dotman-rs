# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_dot_global_optspecs
	string join \n v/verbose h/help V/version
end

function __fish_dot_needs_command
	# Figure out if the current invocation already has a command.
	set -l cmd (commandline -opc)
	set -e cmd[1]
	argparse -s (__fish_dot_global_optspecs) -- $cmd 2>/dev/null
	or return
	if set -q argv[1]
		# Also print the command, so this can be used to figure out what it is.
		echo $argv[1]
		return 1
	end
	return 0
end

function __fish_dot_using_subcommand
	set -l cmd (__fish_dot_needs_command)
	test -z "$cmd"
	and return 1
	contains -- $cmd[1] $argv
end

complete -c dot -n "__fish_dot_needs_command" -s v -l verbose
complete -c dot -n "__fish_dot_needs_command" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c dot -n "__fish_dot_needs_command" -s V -l version -d 'Print version'
complete -c dot -n "__fish_dot_needs_command" -f -a "add" -d 'Add files to be tracked'
complete -c dot -n "__fish_dot_needs_command" -f -a "status" -d 'Show the working tree status'
complete -c dot -n "__fish_dot_needs_command" -f -a "commit" -d 'Record changes to the repository'
complete -c dot -n "__fish_dot_needs_command" -f -a "checkout" -d 'Switch branches or restore working tree files'
complete -c dot -n "__fish_dot_needs_command" -f -a "reset" -d 'Reset current HEAD to the specified state'
complete -c dot -n "__fish_dot_needs_command" -f -a "push" -d 'Update remote refs along with associated objects'
complete -c dot -n "__fish_dot_needs_command" -f -a "pull" -d 'Fetch from and integrate with another repository'
complete -c dot -n "__fish_dot_needs_command" -f -a "init" -d 'Initialize a new dotman repository'
complete -c dot -n "__fish_dot_needs_command" -f -a "show" -d 'Show various types of objects'
complete -c dot -n "__fish_dot_needs_command" -f -a "log" -d 'Show commit logs'
complete -c dot -n "__fish_dot_needs_command" -f -a "diff" -d 'Show changes between commits'
complete -c dot -n "__fish_dot_needs_command" -f -a "rm" -d 'Remove files from tracking'
complete -c dot -n "__fish_dot_needs_command" -f -a "remote" -d 'Manage remote repositories'
complete -c dot -n "__fish_dot_needs_command" -f -a "branch" -d 'Manage branches'
complete -c dot -n "__fish_dot_needs_command" -f -a "config" -d 'Get and set repository or user options'
complete -c dot -n "__fish_dot_needs_command" -f -a "completion" -d 'Generate shell completion scripts'
complete -c dot -n "__fish_dot_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c dot -n "__fish_dot_using_subcommand add" -s f -l force
complete -c dot -n "__fish_dot_using_subcommand add" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand add" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand status" -s s -l short
complete -c dot -n "__fish_dot_using_subcommand status" -s u -l untracked
complete -c dot -n "__fish_dot_using_subcommand status" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand status" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand commit" -s m -l message -r
complete -c dot -n "__fish_dot_using_subcommand commit" -s a -l all
complete -c dot -n "__fish_dot_using_subcommand commit" -l amend -d 'Amend the previous commit'
complete -c dot -n "__fish_dot_using_subcommand commit" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand commit" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand checkout" -s f -l force
complete -c dot -n "__fish_dot_using_subcommand checkout" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand checkout" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand reset" -l hard
complete -c dot -n "__fish_dot_using_subcommand reset" -l soft
complete -c dot -n "__fish_dot_using_subcommand reset" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand reset" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand push" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand push" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand pull" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand pull" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand init" -s b -l bare
complete -c dot -n "__fish_dot_using_subcommand init" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand init" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand show" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand show" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand log" -s n -l limit -r
complete -c dot -n "__fish_dot_using_subcommand log" -l oneline
complete -c dot -n "__fish_dot_using_subcommand log" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand log" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand diff" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand diff" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand rm" -s c -l cached
complete -c dot -n "__fish_dot_using_subcommand rm" -s f -l force
complete -c dot -n "__fish_dot_using_subcommand rm" -s i -l interactive
complete -c dot -n "__fish_dot_using_subcommand rm" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand rm" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand remote; and not __fish_seen_subcommand_from list add remove set-url show rename help" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand remote; and not __fish_seen_subcommand_from list add remove set-url show rename help" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand remote; and not __fish_seen_subcommand_from list add remove set-url show rename help" -f -a "list" -d 'List all remotes'
complete -c dot -n "__fish_dot_using_subcommand remote; and not __fish_seen_subcommand_from list add remove set-url show rename help" -f -a "add" -d 'Add a new remote'
complete -c dot -n "__fish_dot_using_subcommand remote; and not __fish_seen_subcommand_from list add remove set-url show rename help" -f -a "remove" -d 'Remove a remote'
complete -c dot -n "__fish_dot_using_subcommand remote; and not __fish_seen_subcommand_from list add remove set-url show rename help" -f -a "set-url" -d 'Set the URL for a remote'
complete -c dot -n "__fish_dot_using_subcommand remote; and not __fish_seen_subcommand_from list add remove set-url show rename help" -f -a "show" -d 'Show information about a remote'
complete -c dot -n "__fish_dot_using_subcommand remote; and not __fish_seen_subcommand_from list add remove set-url show rename help" -f -a "rename" -d 'Rename a remote'
complete -c dot -n "__fish_dot_using_subcommand remote; and not __fish_seen_subcommand_from list add remove set-url show rename help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c dot -n "__fish_dot_using_subcommand remote; and __fish_seen_subcommand_from list" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand remote; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand remote; and __fish_seen_subcommand_from add" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand remote; and __fish_seen_subcommand_from add" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand remote; and __fish_seen_subcommand_from remove" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand remote; and __fish_seen_subcommand_from remove" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand remote; and __fish_seen_subcommand_from set-url" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand remote; and __fish_seen_subcommand_from set-url" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand remote; and __fish_seen_subcommand_from show" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand remote; and __fish_seen_subcommand_from show" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand remote; and __fish_seen_subcommand_from rename" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand remote; and __fish_seen_subcommand_from rename" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand remote; and __fish_seen_subcommand_from help" -f -a "list" -d 'List all remotes'
complete -c dot -n "__fish_dot_using_subcommand remote; and __fish_seen_subcommand_from help" -f -a "add" -d 'Add a new remote'
complete -c dot -n "__fish_dot_using_subcommand remote; and __fish_seen_subcommand_from help" -f -a "remove" -d 'Remove a remote'
complete -c dot -n "__fish_dot_using_subcommand remote; and __fish_seen_subcommand_from help" -f -a "set-url" -d 'Set the URL for a remote'
complete -c dot -n "__fish_dot_using_subcommand remote; and __fish_seen_subcommand_from help" -f -a "show" -d 'Show information about a remote'
complete -c dot -n "__fish_dot_using_subcommand remote; and __fish_seen_subcommand_from help" -f -a "rename" -d 'Rename a remote'
complete -c dot -n "__fish_dot_using_subcommand remote; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c dot -n "__fish_dot_using_subcommand branch; and not __fish_seen_subcommand_from list create delete rename set-upstream unset-upstream help" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand branch; and not __fish_seen_subcommand_from list create delete rename set-upstream unset-upstream help" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand branch; and not __fish_seen_subcommand_from list create delete rename set-upstream unset-upstream help" -f -a "list" -d 'List all branches'
complete -c dot -n "__fish_dot_using_subcommand branch; and not __fish_seen_subcommand_from list create delete rename set-upstream unset-upstream help" -f -a "create" -d 'Create a new branch'
complete -c dot -n "__fish_dot_using_subcommand branch; and not __fish_seen_subcommand_from list create delete rename set-upstream unset-upstream help" -f -a "delete" -d 'Delete a branch'
complete -c dot -n "__fish_dot_using_subcommand branch; and not __fish_seen_subcommand_from list create delete rename set-upstream unset-upstream help" -f -a "rename" -d 'Rename a branch'
complete -c dot -n "__fish_dot_using_subcommand branch; and not __fish_seen_subcommand_from list create delete rename set-upstream unset-upstream help" -f -a "set-upstream" -d 'Set upstream tracking for a branch'
complete -c dot -n "__fish_dot_using_subcommand branch; and not __fish_seen_subcommand_from list create delete rename set-upstream unset-upstream help" -f -a "unset-upstream" -d 'Remove upstream tracking for a branch'
complete -c dot -n "__fish_dot_using_subcommand branch; and not __fish_seen_subcommand_from list create delete rename set-upstream unset-upstream help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c dot -n "__fish_dot_using_subcommand branch; and __fish_seen_subcommand_from list" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand branch; and __fish_seen_subcommand_from list" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand branch; and __fish_seen_subcommand_from create" -s f -l from -d 'Starting point (commit or branch)' -r
complete -c dot -n "__fish_dot_using_subcommand branch; and __fish_seen_subcommand_from create" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand branch; and __fish_seen_subcommand_from create" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand branch; and __fish_seen_subcommand_from delete" -s f -l force -d 'Force deletion'
complete -c dot -n "__fish_dot_using_subcommand branch; and __fish_seen_subcommand_from delete" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand branch; and __fish_seen_subcommand_from delete" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand branch; and __fish_seen_subcommand_from rename" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand branch; and __fish_seen_subcommand_from rename" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand branch; and __fish_seen_subcommand_from set-upstream" -s b -l branch -d 'Branch name (current branch if not specified)' -r
complete -c dot -n "__fish_dot_using_subcommand branch; and __fish_seen_subcommand_from set-upstream" -s b -l remote-branch -d 'Remote branch name (same as local branch if not specified)' -r
complete -c dot -n "__fish_dot_using_subcommand branch; and __fish_seen_subcommand_from set-upstream" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand branch; and __fish_seen_subcommand_from set-upstream" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand branch; and __fish_seen_subcommand_from unset-upstream" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand branch; and __fish_seen_subcommand_from unset-upstream" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand branch; and __fish_seen_subcommand_from help" -f -a "list" -d 'List all branches'
complete -c dot -n "__fish_dot_using_subcommand branch; and __fish_seen_subcommand_from help" -f -a "create" -d 'Create a new branch'
complete -c dot -n "__fish_dot_using_subcommand branch; and __fish_seen_subcommand_from help" -f -a "delete" -d 'Delete a branch'
complete -c dot -n "__fish_dot_using_subcommand branch; and __fish_seen_subcommand_from help" -f -a "rename" -d 'Rename a branch'
complete -c dot -n "__fish_dot_using_subcommand branch; and __fish_seen_subcommand_from help" -f -a "set-upstream" -d 'Set upstream tracking for a branch'
complete -c dot -n "__fish_dot_using_subcommand branch; and __fish_seen_subcommand_from help" -f -a "unset-upstream" -d 'Remove upstream tracking for a branch'
complete -c dot -n "__fish_dot_using_subcommand branch; and __fish_seen_subcommand_from help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c dot -n "__fish_dot_using_subcommand config" -l unset -d 'Unset the configuration key'
complete -c dot -n "__fish_dot_using_subcommand config" -s l -l list -d 'List all configuration values'
complete -c dot -n "__fish_dot_using_subcommand config" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand config" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand completion" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand completion" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm remote branch config completion help" -f -a "add" -d 'Add files to be tracked'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm remote branch config completion help" -f -a "status" -d 'Show the working tree status'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm remote branch config completion help" -f -a "commit" -d 'Record changes to the repository'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm remote branch config completion help" -f -a "checkout" -d 'Switch branches or restore working tree files'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm remote branch config completion help" -f -a "reset" -d 'Reset current HEAD to the specified state'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm remote branch config completion help" -f -a "push" -d 'Update remote refs along with associated objects'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm remote branch config completion help" -f -a "pull" -d 'Fetch from and integrate with another repository'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm remote branch config completion help" -f -a "init" -d 'Initialize a new dotman repository'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm remote branch config completion help" -f -a "show" -d 'Show various types of objects'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm remote branch config completion help" -f -a "log" -d 'Show commit logs'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm remote branch config completion help" -f -a "diff" -d 'Show changes between commits'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm remote branch config completion help" -f -a "rm" -d 'Remove files from tracking'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm remote branch config completion help" -f -a "remote" -d 'Manage remote repositories'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm remote branch config completion help" -f -a "branch" -d 'Manage branches'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm remote branch config completion help" -f -a "config" -d 'Get and set repository or user options'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm remote branch config completion help" -f -a "completion" -d 'Generate shell completion scripts'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm remote branch config completion help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c dot -n "__fish_dot_using_subcommand help; and __fish_seen_subcommand_from remote" -f -a "list" -d 'List all remotes'
complete -c dot -n "__fish_dot_using_subcommand help; and __fish_seen_subcommand_from remote" -f -a "add" -d 'Add a new remote'
complete -c dot -n "__fish_dot_using_subcommand help; and __fish_seen_subcommand_from remote" -f -a "remove" -d 'Remove a remote'
complete -c dot -n "__fish_dot_using_subcommand help; and __fish_seen_subcommand_from remote" -f -a "set-url" -d 'Set the URL for a remote'
complete -c dot -n "__fish_dot_using_subcommand help; and __fish_seen_subcommand_from remote" -f -a "show" -d 'Show information about a remote'
complete -c dot -n "__fish_dot_using_subcommand help; and __fish_seen_subcommand_from remote" -f -a "rename" -d 'Rename a remote'
complete -c dot -n "__fish_dot_using_subcommand help; and __fish_seen_subcommand_from branch" -f -a "list" -d 'List all branches'
complete -c dot -n "__fish_dot_using_subcommand help; and __fish_seen_subcommand_from branch" -f -a "create" -d 'Create a new branch'
complete -c dot -n "__fish_dot_using_subcommand help; and __fish_seen_subcommand_from branch" -f -a "delete" -d 'Delete a branch'
complete -c dot -n "__fish_dot_using_subcommand help; and __fish_seen_subcommand_from branch" -f -a "rename" -d 'Rename a branch'
complete -c dot -n "__fish_dot_using_subcommand help; and __fish_seen_subcommand_from branch" -f -a "set-upstream" -d 'Set upstream tracking for a branch'
complete -c dot -n "__fish_dot_using_subcommand help; and __fish_seen_subcommand_from branch" -f -a "unset-upstream" -d 'Remove upstream tracking for a branch'
