#!/bin/bash

set -e

printf "Name: "
while IFS= read -r line; do
    echo "$line"
    break;
done
