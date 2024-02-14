#!/bin/bash

set -e

read_password() {
    stty -echo
    read password
    stty echo
    printf "\n"
    echo "$password"
}

printf "Password: "
password=$(read_password)

printf "\nConfirm password: "
password_confirmed=$(read_password)
echo "$confirmed_password"

if [ "$password" == "$password_confirmed" ]; then
    echo "Ok"
else
    echo "Error, passwords do not match"
    exit 1
fi
