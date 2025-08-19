
use builtin;
use str;

set edit:completion:arg-completer[dot] = {|@words|
    fn spaces {|n|
        builtin:repeat $n ' ' | str:join ''
    }
    fn cand {|text desc|
        edit:complex-candidate $text &display=$text' '(spaces (- 14 (wcswidth $text)))$desc
    }
    var command = 'dot'
    for word $words[1..-1] {
        if (str:has-prefix $word '-') {
            break
        }
        set command = $command';'$word
    }
    var completions = [
        &'dot'= {
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help (see more with ''--help'')'
            cand --help 'Print help (see more with ''--help'')'
            cand -V 'Print version'
            cand --version 'Print version'
            cand add 'Add files to be tracked'
            cand status 'Show the working tree status'
            cand commit 'Record changes to the repository'
            cand checkout 'Switch branches or restore working tree files'
            cand reset 'Reset current HEAD to the specified state'
            cand push 'Update remote refs along with associated objects'
            cand pull 'Fetch from and integrate with another repository'
            cand init 'Initialize a new dotman repository'
            cand show 'Show various types of objects'
            cand log 'Show commit logs'
            cand diff 'Show changes between commits'
            cand rm 'Remove files from tracking'
            cand completion 'Generate shell completion scripts'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'dot;add'= {
            cand -f 'f'
            cand --force 'force'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;status'= {
            cand -s 's'
            cand --short 'short'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;commit'= {
            cand -m 'm'
            cand --message 'message'
            cand -a 'a'
            cand --all 'all'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;checkout'= {
            cand -f 'f'
            cand --force 'force'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;reset'= {
            cand --hard 'hard'
            cand --soft 'soft'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;push'= {
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;pull'= {
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;init'= {
            cand -b 'b'
            cand --bare 'bare'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;show'= {
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;log'= {
            cand -n 'n'
            cand --limit 'limit'
            cand --oneline 'oneline'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;diff'= {
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;rm'= {
            cand -c 'c'
            cand --cached 'cached'
            cand -f 'f'
            cand --force 'force'
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;completion'= {
            cand -v 'v'
            cand --verbose 'verbose'
            cand -h 'Print help'
            cand --help 'Print help'
        }
        &'dot;help'= {
            cand add 'Add files to be tracked'
            cand status 'Show the working tree status'
            cand commit 'Record changes to the repository'
            cand checkout 'Switch branches or restore working tree files'
            cand reset 'Reset current HEAD to the specified state'
            cand push 'Update remote refs along with associated objects'
            cand pull 'Fetch from and integrate with another repository'
            cand init 'Initialize a new dotman repository'
            cand show 'Show various types of objects'
            cand log 'Show commit logs'
            cand diff 'Show changes between commits'
            cand rm 'Remove files from tracking'
            cand completion 'Generate shell completion scripts'
            cand help 'Print this message or the help of the given subcommand(s)'
        }
        &'dot;help;add'= {
        }
        &'dot;help;status'= {
        }
        &'dot;help;commit'= {
        }
        &'dot;help;checkout'= {
        }
        &'dot;help;reset'= {
        }
        &'dot;help;push'= {
        }
        &'dot;help;pull'= {
        }
        &'dot;help;init'= {
        }
        &'dot;help;show'= {
        }
        &'dot;help;log'= {
        }
        &'dot;help;diff'= {
        }
        &'dot;help;rm'= {
        }
        &'dot;help;completion'= {
        }
        &'dot;help;help'= {
        }
    ]
    $completions[$command]
}
