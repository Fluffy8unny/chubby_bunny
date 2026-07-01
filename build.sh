#!/usr/bin/env sh
set -eu

sh chubby_bunny_playground/build.sh
sh examples/minimal_box/build.sh
sh examples/constraint_example/build.sh
sh examples/svg_example/build.sh
sh examples/interactive_example/build.sh

python3 -m http.server 8000