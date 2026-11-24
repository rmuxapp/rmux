# Source from an interactive Bash session or ~/.bashrc. Bash 3.2+; no external commands.
[[ $- == *i* && ${TERM_PROGRAM-} == rmux ]] || return 0
[[ ${RMUX_SHELL_INTEGRATION-1} != 0 && -z ${_rmux_bash_loaded-} ]] || return 0

_rmux_bash_loaded=1
_rmux_bash_directory_path=''
_rmux_bash_directory_encoded=''
# Bash 4.4+ expands PS0 after reading a command and before executing it.
# Preserve user pre-command prompt content without installing a DEBUG trap.
# https://www.gnu.org/software/bash/manual/html_node/Bash-Variables.html
if (( BASH_VERSINFO[0] > 4 || (BASH_VERSINFO[0] == 4 && BASH_VERSINFO[1] >= 4) )); then
    PS0=$'\e]133;C\a'${PS0-}
fi
_rmux_bash_prompt_end=$'\\[\e]133;B\a\\]'

# Run before user prompt hooks, which may overwrite the command's exit status.
# Returning it also preserves the status seen by the first user hook.
_rmux_bash_capture_status() {
    _rmux_bash_command_status=$?
    return "$_rmux_bash_command_status"
}


# Only shell-approved history is eligible; never inspect raw terminal input.
# Kept off until the application enables the complete persistence lifecycle.
_rmux_bash_history_report() {
    [[ ${RMUX_SESSION_HISTORY-0} == 1 && -n ${HISTFILE-} && $HISTFILE != /dev/null ]] || return 0
    [[ -o history ]] || return 0
    if [[ -n ${RMUX_HISTORY_IMPORT-} ]]; then
        local import_file=$RMUX_HISTORY_IMPORT
        unset RMUX_HISTORY_IMPORT
        # Import as history data, never source/eval or terminal input.
        if [[ -r $import_file ]]; then
            # Flush genuinely new entries first. Mark the imported entries as
            # already written so normal exit/history -a does not duplicate them.
            builtin history -a || return 0
            local imported_command
            while IFS= read -r -d '' imported_command; do
                builtin history -s -- "$imported_command"
            done < "$import_file"
            builtin history -a /dev/null
        fi
    fi
    local listing sequence command LC_ALL=C encoded='' character hex byte index
    listing=$(HISTTIMEFORMAT= builtin history 1)
    if [[ -z $listing ]]; then
        if [[ -n ${_rmux_history_sequence-} && $_rmux_history_sequence != 0 ]]; then
            builtin printf '\e]9999;rmux-history;clear\a'
        fi
        _rmux_history_sequence=0
        _rmux_history_command=''
        return 0
    fi
    [[ $listing =~ ^[[:space:]]*([0-9]+)[*]?[[:space:]][[:space:]](.*)$ ]] || return 0
    sequence=${BASH_REMATCH[1]}
    command=${BASH_REMATCH[2]}
    if [[ -z ${_rmux_history_sequence+x} ]]; then
        _rmux_history_sequence=$sequence
        (( ${#command} <= 8192 )) && _rmux_history_command=$command
        return 0
    fi
    [[ $sequence != $_rmux_history_sequence || $command != ${_rmux_history_command-} ]] || return 0
    _rmux_history_sequence=$sequence
    (( ${#command} > 0 && ${#command} <= 8192 )) || { _rmux_history_command=''; return 0; }
    _rmux_history_command=$command
    for (( index=0; index<${#command}; index++ )); do
        character=${command:index:1}
        builtin printf -v byte '%d' "'$character"
        builtin printf -v hex '%02x' "$((byte & 255))"
        encoded+=$hex
    done
    builtin printf '\e]9999;rmux-history;%s\a' "$encoded"
}

_rmux_bash_precmd() {
    local command_status=${_rmux_bash_command_status-0}
    _rmux_bash_history_report
    if [[ ${_rmux_bash_prompt_seen-} == 1 ]]; then
        builtin printf '\e]133;D;%d\a' "$command_status"
    fi
    _rmux_bash_prompt_seen=1
    if [[ $_rmux_bash_directory_path != "$PWD" ]]; then
        local LC_ALL=C
        local encoded='' character hex byte index
        for (( index = 0; index < ${#PWD}; index++ )); do
            character=${PWD:index:1}
            case $character in
                [a-zA-Z0-9/._~-]) encoded+=$character ;;
                *) builtin printf -v byte '%d' "'$character"
                   builtin printf -v hex '%02X' "$((byte & 255))"
                   encoded+="%$hex" ;;
            esac
        done
        _rmux_bash_directory_path=$PWD
        _rmux_bash_directory_encoded=$encoded
    fi
    builtin printf '\e]7;file://localhost%s\a\e]133;A\a' "$_rmux_bash_directory_encoded"
    PS1=${PS1%"$_rmux_bash_prompt_end"}$_rmux_bash_prompt_end
    return "$command_status"
}

# Preserve scalar hooks, including multiline commands and trailing comments.
# Modern Bash also supports PROMPT_COMMAND arrays; retain their element order.
# Other attributes (for example export: -ax) must not turn an indexed array
# into the scalar path, which would overwrite element zero and repeat hooks.
if [[ $(declare -p PROMPT_COMMAND 2>/dev/null) =~ ^declare[[:space:]]+-[^[:space:]]*a[^[:space:]]*[[:space:]] ]]; then
    PROMPT_COMMAND=(_rmux_bash_capture_status "${PROMPT_COMMAND[@]}" _rmux_bash_precmd)
else
    PROMPT_COMMAND=$'_rmux_bash_capture_status\n'"${PROMPT_COMMAND-}"$'\n_rmux_bash_precmd'
fi

# Preserve existing user aliases/functions and allow an explicit opt-out.
if [[ -n ${RMUX_CLI-} && ${RMUX_CLAUDE_INTEGRATION-1} != 0 ]] && ! alias claude >/dev/null 2>&1 && ! typeset -f claude >/dev/null 2>&1; then
    function claude { "$RMUX_CLI" claude "$@"; }
fi
