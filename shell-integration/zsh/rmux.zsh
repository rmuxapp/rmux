# Source from an interactive zsh session or ~/.zshrc. No external commands.
[[ -o interactive && ${TERM_PROGRAM-} == rmux ]] || return 0
[[ ${RMUX_SHELL_INTEGRATION-1} != 0 && -z ${_rmux_integration_loaded-} ]] || return 0

typeset -g _rmux_integration_loaded=1
typeset -g _rmux_command_running=0
typeset -g _rmux_prompt_end=$'%{\e]133;B\a%}'

_rmux_report_directory() {
    emulate -L zsh
    # Encode bytes, including UTF-8, rather than Unicode code points. Keeping
    # controls encoded prevents paths from terminating or injecting OSCs.
    local LC_ALL=C
    local encoded='' character hex byte
    local -i index
    for (( index = 1; index <= ${#PWD}; index++ )); do
        character=${PWD[index]}
        case $character in
            [a-zA-Z0-9/._~-]) encoded+=$character ;;
            *) builtin printf -v hex '%02X' "'$character"
               encoded+="%$hex" ;;
        esac
    done
    builtin printf '\e]7;file://localhost%s\a' "$encoded"
}


# Keep capture tied to the shell's history policy, rather than preexec input.
_rmux_zsh_history_report() {
    [[ ${RMUX_SESSION_HISTORY-0} == 1 && -n ${HISTFILE-} && $HISTFILE != /dev/null ]] || return 0
    (( ${SAVEHIST:-0} > 0 )) || return 0
    local listing sequence command encoded='' character hex byte
    local -i index
    listing=$(builtin fc -l -1 2>/dev/null)
    [[ $listing =~ '^ *([0-9]+)' ]] || return 0
    sequence=$match[1]
    if [[ -z ${_rmux_history_sequence+x} ]]; then
        typeset -g _rmux_history_sequence=$sequence
        return 0
    fi
    [[ $sequence != $_rmux_history_sequence ]] || return 0
    zmodload -F zsh/parameter p:history 2>/dev/null || return 0
    command=${history[$sequence]}
    [[ -o hist_ignore_space && $command == ' '* ]] && return 0
    typeset -g _rmux_history_sequence=$sequence
    local LC_ALL=C
    (( ${#command} > 0 && ${#command} <= 8192 )) || return 0
    for (( index=1; index<=${#command}; index++ )); do
        character=${command[index]}
        builtin printf -v byte '%d' "'$character"
        builtin printf -v hex '%02x' "$((byte & 255))"
        encoded+=$hex
    done
    builtin printf '\e]9999;rmux-history;%s\a' "$encoded"
}

_rmux_precmd() {
    local command_status=$?
    _rmux_zsh_history_report
    emulate -L zsh
    if (( _rmux_command_running )); then
        builtin printf '\e]133;D;%d\a' "$command_status"
        _rmux_command_running=0
    fi
    _rmux_report_directory
    builtin printf '\e]133;A\a'
    # Themes may replace PS1 on each prompt. Strip only our prior suffix and
    # append one zero-width input marker after the final visible prompt.
    PS1=${PS1%"$_rmux_prompt_end"}$_rmux_prompt_end
}

_rmux_preexec() {
    emulate -L zsh
    _rmux_command_running=1
    builtin printf '\e]133;C\a'
}

# Preserve existing hooks and avoid evaluating command text or changing traps.
typeset -ga precmd_functions preexec_functions
precmd_functions+=(_rmux_precmd)
preexec_functions+=(_rmux_preexec)
