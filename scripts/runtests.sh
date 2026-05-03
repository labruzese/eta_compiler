#!/usr/bin/env bash

SHARED_PATH=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)

TESTS="shared/tests"

# run tests
"$SHARED_PATH/scripts/drun.sh" ./eth/eth "$@" --compilerpath shared/scripts --testpath "$TESTS" ./shared/scripts/ethScript
