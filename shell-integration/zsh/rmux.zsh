# Source from an interactive zsh session or ~/.zshrc. No external commands.
[[ -o interactive && ${TERM_PROGRAM-} == rmux ]] || return 0
[[ ${RMUX_SHELL_INTEGRATION-1} != 0 && -z ${_rmux_integration_loaded-} ]] || return 0

typeset -g _rmux_integration_loaded=1
typeset -g _rmux_command_running=0
typeset -g _rmux_directory_path='' _rmux_directory_encoded=''
typeset -g _rmux_prompt_end=$'%{\e]133;B\a%}'

_rmux_report_directory() {
    emulate -L zsh
    # Encode bytes, including UTF-8, rather than Unicode code points. Keeping
    # controls encoded prevents paths from terminating or injecting OSCs.
    if [[ $_rmux_directory_path != "$PWD" ]]; then
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
        _rmux_directory_path=$PWD
        _rmux_directory_encoded=$encoded
    fi
    builtin printf '\e]7;file://localhost%s\a' "$_rmux_directory_encoded"
}


# Keep capture tied to the shell's history policy, rather than preexec input.
_rmux_zsh_history_report() {
    [[ ${RMUX_SESSION_HISTORY-0} == 1 && -n ${HISTFILE-} && $HISTFILE != /dev/null ]] || return 0
    (( ${SAVEHIST:-0} > 0 )) || return 0
    if [[ -n ${RMUX_HISTORY_IMPORT-} ]]; then
        local import_file=$RMUX_HISTORY_IMPORT
        unset RMUX_HISTORY_IMPORT
        # Import as history data, never source/eval or terminal input.
        [[ ! -r $import_file ]] || builtin fc -R "$import_file"
    fi
    local listing sequence command encoded='' character hex byte
    local -i index
    zmodload -F zsh/parameter p:history 2>/dev/null || return 0
    # fc prints a numbered blank row for an empty history stack.
    if (( ${#history} == 0 )); then listing=''
    else listing=$(builtin fc -l -1 2>/dev/null); fi
    if [[ -z $listing ]]; then
        if [[ -n ${_rmux_history_sequence-} && $_rmux_history_sequence != 0 ]]; then
            builtin printf '\e]9999;rmux-history;clear\a'
        fi
        typeset -g _rmux_history_sequence=0
        typeset -g _rmux_history_command=''
        return 0
    fi
    [[ $listing =~ '^ *([0-9]+)' ]] || return 0
    sequence=$match[1]
    command=${history[$sequence]}
    local LC_ALL=C
    if [[ -z ${_rmux_history_sequence+x} ]]; then
        typeset -g _rmux_history_sequence=$sequence
        (( ${#command} <= 8192 )) && typeset -g _rmux_history_command=$command
        return 0
    fi
    [[ $sequence != $_rmux_history_sequence || $command != ${_rmux_history_command-} ]] || return 0
    [[ -o hist_ignore_space && $command == ' '* ]] && return 0
    typeset -g _rmux_history_sequence=$sequence
    (( ${#command} > 0 && ${#command} <= 8192 )) || { typeset -g _rmux_history_command=''; return 0; }
    typeset -g _rmux_history_command=$command
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

# Preserve existing user aliases/functions and allow an explicit opt-out.
if [[ -n ${RMUX_CLI-} && ${RMUX_CLAUDE_INTEGRATION-1} != 0 ]] && ! alias claude >/dev/null 2>&1 && ! typeset -f claude >/dev/null 2>&1; then
    function claude { "$RMUX_CLI" claude "$@"; }
fi
