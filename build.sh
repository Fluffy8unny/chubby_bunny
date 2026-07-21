#!/usr/bin/env sh
set -eu

ENABLE_PROFILING=0
while [ "$#" -gt 0 ]; do
	case "$1" in
		--enable_profiling)
			ENABLE_PROFILING=1
			;;
		-h|--help)
			printf '%s\n' "Usage: $0 [--enable_profiling]"
			exit 0
			;;
		*)
			printf '%s\n' "Unknown argument: $1" >&2
			exit 1
			;;
	esac
	shift
done

PROFILE_ARG=""
if [ "$ENABLE_PROFILING" -eq 1 ]; then
	PROFILE_ARG="--enable_profiling"
fi

sh chubby_bunny_playground/build.sh $PROFILE_ARG
sh examples/minimal_box/build.sh $PROFILE_ARG
sh examples/constraint_example/build.sh $PROFILE_ARG
sh examples/svg_example/build.sh $PROFILE_ARG
sh examples/interactive_example/build.sh $PROFILE_ARG

python3 -m http.server 8000