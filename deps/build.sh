#!/bin/sh
# Fetches mhttp's native dependencies at pinned versions, applies the patches in
# patches/ and builds them as static libraries. Prints the variables cargo needs.
#
#   eval "$(deps/build.sh)"
#   cargo build
#
# Needs git, cmake, ninja, a C/C++20 compiler and perl (BoringSSL).
set -eu

cd "$(dirname "$0")"
deps=$(pwd)
mkdir -p src

fetch() { # name url revision destination
	if [ ! -d "$4/.git" ]; then
		git clone -q --filter=blob:none "$2" "$4" >&2
	fi
	git -C "$4" checkout -q "$3"
}

patch() { # destination patch
	if git -C "$1" apply --reverse --check "$deps/patches/$2" 2>/dev/null; then
		return # already applied
	fi
	git -C "$1" apply "$deps/patches/$2"
}

build() { # source-dir extra-cmake-args...
	dir=$1
	shift
	cmake -S "$dir" -B "$dir/build" -G Ninja -DCMAKE_BUILD_TYPE=Release \
		-DCMAKE_POSITION_INDEPENDENT_CODE=ON "$@" >&2
	cmake --build "$dir/build" >&2
}

fetch boringssl https://boringssl.googlesource.com/boringssl 148a7b1f361dcf6708e9eb3b7c2134d0c5633e3e src/boringssl
fetch nghttp2 https://github.com/nghttp2/nghttp2.git v1.71.0 src/nghttp2
fetch libuv https://github.com/libuv/libuv.git v1.53.0 src/libuv
fetch cppcoro https://github.com/andreasbuhr/cppcoro.git 8642e98596a92be30a2b061d3ed306d959d3214e src/cppcoro
fetch cxx-async https://github.com/pcwalton/cxx-async.git 1575822aa7fcf484b37f43b52e60bb168b259760 cxx-async

patch src/boringssl boringssl.patch
patch src/nghttp2 nghttp2.patch
patch cxx-async cxx-async.patch

# newer gcc flags a false stringop-overflow in boringssl, which builds with -Werror
boringssl_flags=""
if ${CXX:-c++} --version 2>/dev/null | grep -qi 'free software foundation'; then
	boringssl_flags="-DCMAKE_C_FLAGS=-Wno-stringop-overflow -DCMAKE_CXX_FLAGS=-Wno-stringop-overflow"
fi

# shellcheck disable=SC2086
build src/boringssl -DBUILD_TESTING=OFF $boringssl_flags
build src/nghttp2 -DENABLE_LIB_ONLY=ON -DBUILD_SHARED_LIBS=OFF -DBUILD_STATIC_LIBS=ON -DBUILD_TESTING=OFF
build src/libuv -DLIBUV_BUILD_SHARED=OFF -DLIBUV_BUILD_TESTS=OFF
build src/cppcoro -DBUILD_TESTING=OFF -DCMAKE_CXX_STANDARD=20

echo "export BORINGSSL_DIR=$deps/src/boringssl"
echo "export NGHTTP2_DIR=$deps/src/nghttp2"
echo "export LIBUV_DIR=$deps/src/libuv"
echo "export CPPCORO_DIR=$deps/src/cppcoro"
