#!/bin/bash

SHARED_PATH=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

# run tests
"$SHARED_PATH/scripts/drun.sh" ./eth/eth --compilerpath shared/scripts --testpath ./eth/tests/pa1 ./shared/scripts/ethScript
