#!/usr/bin/env bash

SHARED_PATH=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
docker run -it -v "$SHARED_PATH":/home/student/shared charlessherk/cs4120-vm
