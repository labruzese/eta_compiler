#!/usr/bin/env bash

### Script to start our docker vm.
### Will run the docker image with the provided command or start interactive if none

# shared path is one level above this
SHARED_PATH=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

if ! docker image inspect eta-vm >/dev/null 2>&1; then
    docker build -t eta-vm "$SHARED_PATH/scripts"
fi

if [ "$#" -eq 0 ]; then 
	docker run -it -v "$SHARED_PATH":/home/student/shared eta-vm
else 
	docker run -v "$SHARED_PATH":/home/student/shared eta-vm "$@"
fi
