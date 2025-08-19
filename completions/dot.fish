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
complete -c dot -n "__fish_dot_needs_command" -f -a "completion" -d 'Generate shell completion scripts'
complete -c dot -n "__fish_dot_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c dot -n "__fish_dot_using_subcommand add" -s f -l force
complete -c dot -n "__fish_dot_using_subcommand add" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand add" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand status" -s s -l short
complete -c dot -n "__fish_dot_using_subcommand status" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand status" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand commit" -s m -l message -r
complete -c dot -n "__fish_dot_using_subcommand commit" -s a -l all
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
complete -c dot -n "__fish_dot_using_subcommand rm" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand rm" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand completion" -s v -l verbose
complete -c dot -n "__fish_dot_using_subcommand completion" -s h -l help -d 'Print help'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm completion help" -f -a "add" -d 'Add files to be tracked'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm completion help" -f -a "status" -d 'Show the working tree status'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm completion help" -f -a "commit" -d 'Record changes to the repository'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm completion help" -f -a "checkout" -d 'Switch branches or restore working tree files'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm completion help" -f -a "reset" -d 'Reset current HEAD to the specified state'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm completion help" -f -a "push" -d 'Update remote refs along with associated objects'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm completion help" -f -a "pull" -d 'Fetch from and integrate with another repository'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm completion help" -f -a "init" -d 'Initialize a new dotman repository'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm completion help" -f -a "show" -d 'Show various types of objects'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm completion help" -f -a "log" -d 'Show commit logs'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm completion help" -f -a "diff" -d 'Show changes between commits'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm completion help" -f -a "rm" -d 'Remove files from tracking'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm completion help" -f -a "completion" -d 'Generate shell completion scripts'
complete -c dot -n "__fish_dot_using_subcommand help; and not __fish_seen_subcommand_from add status commit checkout reset push pull init show log diff rm completion help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
