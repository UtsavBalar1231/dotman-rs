#!/usr/bin/env bash
# Enhanced Bash completion for dotman (dot)
# Provides dynamic completions for branches, remotes, commits, config keys, and files

# Helper function to get branches
_dot_get_branches() {
    if [[ -d "$HOME/.dotman" ]]; then
        dot branch list 2>/dev/null | grep '^\*\|^  ' | sed 's/^[* ] //' | grep -v '^$'
    fi
}

# Helper function to get remotes
_dot_get_remotes() {
    if [[ -d "$HOME/.dotman" ]]; then
        dot remote list 2>/dev/null | grep -v '^$'
    fi
}

# Helper function to get recent commits (last 20)
_dot_get_commits() {
    if [[ -d "$HOME/.dotman" ]]; then
        # Get commit IDs and their first line of message
        dot log --oneline -n 20 2>/dev/null | awk '{print $1}'
    fi
}

# Helper function to get config keys
_dot_get_config_keys() {
    echo "user.name user.email"
    echo "core.compression core.compression_level core.default_branch"
    echo "performance.parallel_threads performance.mmap_threshold performance.cache_size performance.use_hard_links"
    echo "tracking.follow_symlinks tracking.preserve_permissions"
}

# Helper function to get untracked files
_dot_get_untracked_files() {
    if [[ -d "$HOME/.dotman" ]]; then
        dot status --untracked 2>/dev/null | grep '^?' | awk '{print $2}'
    fi
}

# Helper function to get tracked files
_dot_get_tracked_files() {
    if [[ -d "$HOME/.dotman" ]]; then
        dot status 2>/dev/null | grep -E '^(M|A|D)' | awk '{print $2}'
    fi
}

# Helper function to get current branch
_dot_get_current_branch() {
    if [[ -d "$HOME/.dotman" ]]; then
        dot branch list 2>/dev/null | grep '^\*' | sed 's/^[* ] //'
    fi
}

# Main enhanced completion function
_dot_enhanced() {
    local cur prev words cword
    if type -t _init_completion >/dev/null; then
        _init_completion || return
    else
        # Fallback for older bash-completion
        cur="${COMP_WORDS[COMP_CWORD]}"
        prev="${COMP_WORDS[COMP_CWORD-1]}"
        words=("${COMP_WORDS[@]}")
        cword=$COMP_CWORD
    fi

    # Get the command structure
    local cmd=""
    local subcmd=""
    local i
    for ((i=1; i < cword; i++)); do
        if [[ "${words[i]}" != -* ]]; then
            if [[ -z "$cmd" ]]; then
                cmd="${words[i]}"
            elif [[ -z "$subcmd" ]]; then
                subcmd="${words[i]}"
                break
            fi
        fi
    done

    # Handle main command completions
    if [[ $cword -eq 1 ]]; then
        COMPREPLY=($(compgen -W "add status commit checkout reset push pull init show log diff rm remote branch config completion help" -- "$cur"))
        return 0
    fi

    # Handle subcommand and dynamic completions
    case "$cmd" in
        checkout)
            # Complete with branches and recent commits
            if [[ "$cur" != -* ]]; then
                local branches=$(_dot_get_branches)
                local commits=$(_dot_get_commits)
                COMPREPLY=($(compgen -W "$branches $commits HEAD" -- "$cur"))
            else
                COMPREPLY=($(compgen -W "-f --force -h --help" -- "$cur"))
            fi
            ;;

        branch)
            case "$subcmd" in
                delete|rename)
                    # Complete with branch names (excluding current branch for delete)
                    if [[ "$cur" != -* ]]; then
                        local branches=$(_dot_get_branches)
                        if [[ "$subcmd" == "delete" ]]; then
                            # Exclude current branch
                            local current=$(_dot_get_current_branch)
                            branches=$(echo "$branches" | grep -v "^$current$")
                        fi
                        COMPREPLY=($(compgen -W "$branches" -- "$cur"))
                    else
                        if [[ "$subcmd" == "delete" ]]; then
                            COMPREPLY=($(compgen -W "-f --force -h --help" -- "$cur"))
                        else
                            COMPREPLY=($(compgen -W "-h --help" -- "$cur"))
                        fi
                    fi
                    ;;
                create)
                    if [[ "$prev" == "--from" || "$prev" == "-f" ]]; then
                        # Complete with branches and commits for --from
                        local branches=$(_dot_get_branches)
                        local commits=$(_dot_get_commits)
                        COMPREPLY=($(compgen -W "$branches $commits HEAD" -- "$cur"))
                    elif [[ "$cur" == -* ]]; then
                        COMPREPLY=($(compgen -W "-f --from -h --help" -- "$cur"))
                    fi
                    ;;
                set-upstream)
                    if [[ "$prev" == "set-upstream" ]]; then
                        # Complete with remote names
                        local remotes=$(_dot_get_remotes)
                        COMPREPLY=($(compgen -W "$remotes" -- "$cur"))
                    elif [[ "$prev" == "--branch" || "$prev" == "-b" ]]; then
                        # Complete with branch names
                        local branches=$(_dot_get_branches)
                        COMPREPLY=($(compgen -W "$branches" -- "$cur"))
                    elif [[ "$cur" == -* ]]; then
                        COMPREPLY=($(compgen -W "-b --branch --remote-branch -h --help" -- "$cur"))
                    fi
                    ;;
                unset-upstream)
                    if [[ "$cur" != -* ]]; then
                        # Complete with branch names
                        local branches=$(_dot_get_branches)
                        COMPREPLY=($(compgen -W "$branches" -- "$cur"))
                    else
                        COMPREPLY=($(compgen -W "-h --help" -- "$cur"))
                    fi
                    ;;
                "")
                    # No subcommand yet
                    COMPREPLY=($(compgen -W "list create delete rename set-upstream unset-upstream" -- "$cur"))
                    ;;
            esac
            ;;

        remote)
            case "$subcmd" in
                remove|show|rename|set-url)
                    # Complete with remote names
                    if [[ "$cur" != -* ]]; then
                        local remotes=$(_dot_get_remotes)
                        COMPREPLY=($(compgen -W "$remotes" -- "$cur"))
                    else
                        COMPREPLY=($(compgen -W "-h --help" -- "$cur"))
                    fi
                    ;;
                "")
                    # No subcommand yet
                    COMPREPLY=($(compgen -W "list add remove set-url show rename" -- "$cur"))
                    ;;
            esac
            ;;

        push|pull)
            # Complete with remotes for first arg, branches for second
            local arg_num=1
            for ((i=1; i < cword; i++)); do
                if [[ "${words[i]}" == "$cmd" ]]; then
                    arg_num=$((cword - i))
                    break
                fi
            done

            if [[ $arg_num -eq 1 ]] && [[ "$cur" != -* ]]; then
                # First argument: remote
                local remotes=$(_dot_get_remotes)
                COMPREPLY=($(compgen -W "$remotes" -- "$cur"))
            elif [[ $arg_num -eq 2 ]] && [[ "$cur" != -* ]]; then
                # Second argument: branch
                local branches=$(_dot_get_branches)
                COMPREPLY=($(compgen -W "$branches" -- "$cur"))
            elif [[ "$cur" == -* ]]; then
                COMPREPLY=($(compgen -W "-h --help" -- "$cur"))
            fi
            ;;

        config)
            if [[ $cword -eq 2 ]] || ([[ $cword -eq 3 ]] && [[ "${words[2]}" == --* ]]); then
                # Complete with config keys
                local keys=$(_dot_get_config_keys)
                COMPREPLY=($(compgen -W "$keys" -- "$cur"))
            elif [[ "$cur" == -* ]]; then
                COMPREPLY=($(compgen -W "--unset --list -h --help" -- "$cur"))
            fi

            # Smart completion for partial keys
            if [[ "$cur" == user.* ]]; then
                COMPREPLY=($(compgen -W "user.name user.email" -- "$cur"))
            elif [[ "$cur" == core.* ]]; then
                COMPREPLY=($(compgen -W "core.compression core.compression_level core.default_branch" -- "$cur"))
            elif [[ "$cur" == performance.* ]]; then
                COMPREPLY=($(compgen -W "performance.parallel_threads performance.mmap_threshold performance.cache_size performance.use_hard_links" -- "$cur"))
            elif [[ "$cur" == tracking.* ]]; then
                COMPREPLY=($(compgen -W "tracking.follow_symlinks tracking.preserve_permissions" -- "$cur"))
            fi
            ;;

        add)
            if [[ "$cur" != -* ]]; then
                # Complete with untracked files and directories
                # First try to get untracked files from dot status
                local untracked=$(_dot_get_untracked_files)
                if [[ -n "$untracked" ]]; then
                    COMPREPLY=($(compgen -W "$untracked" -- "$cur"))
                fi
                # Also complete with filesystem paths
                COMPREPLY+=($(compgen -f -- "$cur"))
            else
                COMPREPLY=($(compgen -W "-f --force -h --help" -- "$cur"))
            fi
            ;;

        rm)
            if [[ "$prev" == "--cached" || "$prev" == "-c" ]]; then
                # Complete with tracked files
                local tracked=$(_dot_get_tracked_files)
                COMPREPLY=($(compgen -W "$tracked" -- "$cur"))
            elif [[ "$cur" != -* ]]; then
                # Complete with all files
                COMPREPLY=($(compgen -f -- "$cur"))
            else
                COMPREPLY=($(compgen -W "-c --cached -f --force -i --interactive -h --help" -- "$cur"))
            fi
            ;;

        show|diff|reset)
            if [[ "$cur" != -* ]]; then
                # Complete with commits and HEAD
                local commits=$(_dot_get_commits)
                local branches=$(_dot_get_branches)
                COMPREPLY=($(compgen -W "$commits $branches HEAD" -- "$cur"))
            else
                case "$cmd" in
                    reset)
                        COMPREPLY=($(compgen -W "--hard --soft -h --help" -- "$cur"))
                        ;;
                    diff)
                        COMPREPLY=($(compgen -W "-h --help" -- "$cur"))
                        ;;
                    show)
                        COMPREPLY=($(compgen -W "-h --help" -- "$cur"))
                        ;;
                esac
            fi
            ;;

        log)
            if [[ "$prev" == "--limit" || "$prev" == "-n" ]]; then
                # Suggest common limit values
                COMPREPLY=($(compgen -W "5 10 20 50 100" -- "$cur"))
            elif [[ "$cur" != -* ]]; then
                # Complete with branches and commits
                local branches=$(_dot_get_branches)
                local commits=$(_dot_get_commits)
                COMPREPLY=($(compgen -W "$branches $commits HEAD" -- "$cur"))
            else
                COMPREPLY=($(compgen -W "-n --limit --oneline -h --help" -- "$cur"))
            fi
            ;;

        commit)
            if [[ "$prev" == "--message" || "$prev" == "-m" ]]; then
                # Don't complete message content
                COMPREPLY=()
            elif [[ "$cur" == -* ]]; then
                COMPREPLY=($(compgen -W "-m --message -a --all --amend -h --help" -- "$cur"))
            fi
            ;;

        status)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=($(compgen -W "-s --short -u --untracked -h --help" -- "$cur"))
            fi
            ;;

        init)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=($(compgen -W "-b --bare -h --help" -- "$cur"))
            fi
            ;;

        completion)
            if [[ "$cur" != -* ]]; then
                COMPREPLY=($(compgen -W "bash zsh fish powershell elvish" -- "$cur"))
            else
                COMPREPLY=($(compgen -W "-h --help" -- "$cur"))
            fi
            ;;
    esac
}

# Register the enhanced completion function
complete -F _dot_enhanced dot

# Also preserve the basic completion as a fallback
if [[ -f "${BASH_SOURCE[0]%/*}/dot.bash" ]]; then
    # Source the basic completion but don't let it override our enhanced one
    TEMP_COMPLETE=$(complete -p dot 2>/dev/null)
    source "${BASH_SOURCE[0]%/*}/dot.bash" 2>/dev/null || true
    complete -F _dot_enhanced dot
fi
