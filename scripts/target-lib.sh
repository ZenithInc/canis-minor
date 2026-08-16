#!/usr/bin/env bash

# Populates SSH_DESTINATION and SSH_PORT from one non-comment line. Accepted form:
#   ssh -p 2222 user@192.0.2.10
# Options may appear in either order. No token is evaluated as shell input.
parse_ssh_target() {
    local target_file=${1:?target file is required}
    local line token destination="" port="22" expect_port=0 saw_ssh=0
    local -a tokens octets _target_lines

    if [[ ! -r "$target_file" ]]; then
        echo "cannot read SSH target file: $target_file" >&2
        return 1
    fi
    mapfile -t _target_lines < <(sed 's/[[:space:]]*#.*$//' "$target_file" | sed '/^[[:space:]]*$/d')
    if [[ ${#_target_lines[@]} -ne 1 ]]; then
        echo "SSH target file must contain exactly one non-comment line" >&2
        return 1
    fi
    line=${_target_lines[0]}
    read -r -a tokens <<<"$line"
    for token in "${tokens[@]}"; do
        if (( expect_port )); then
            [[ $token =~ ^[0-9]{1,5}$ ]] || {
                echo "invalid SSH port" >&2
                return 1
            }
            port=$token
            expect_port=0
            continue
        fi
        case "$token" in
            ssh)
                (( saw_ssh == 0 )) || {
                    echo "duplicate ssh token" >&2
                    return 1
                }
                saw_ssh=1
                ;;
            -p)
                expect_port=1
                ;;
            *@*)
                [[ -z $destination ]] || {
                    echo "multiple SSH destinations are not allowed" >&2
                    return 1
                }
                destination=$token
                ;;
            *)
                echo "unsupported token in SSH target file" >&2
                return 1
                ;;
        esac
    done
    (( expect_port == 0 )) || {
        echo "missing value after -p" >&2
        return 1
    }
    [[ $destination =~ ^[A-Za-z_][A-Za-z0-9_.-]*@[0-9]+[.][0-9]+[.][0-9]+[.][0-9]+$ ]] || {
        echo "SSH destination must be user@IPv4" >&2
        return 1
    }
    (( 10#$port >= 1 && 10#$port <= 65535 )) || {
        echo "SSH port is outside 1..65535" >&2
        return 1
    }

    IFS=. read -r -a octets <<<"${destination#*@}"
    [[ ${#octets[@]} -eq 4 ]] || return 1
    for token in "${octets[@]}"; do
        if [[ ! $token =~ ^[0-9]{1,3}$ ]] || (( 10#$token > 255 )); then
            echo "invalid IPv4 address in SSH destination" >&2
            return 1
        fi
    done

    SSH_DESTINATION=$destination
    SSH_PORT=$((10#$port))
    export SSH_DESTINATION SSH_PORT
}
