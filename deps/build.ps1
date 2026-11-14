# windows version of build.sh, run from a visual studio developer prompt (pwsh):
#
#   . deps\build.ps1
#   cargo build
#
# needs git, cmake and ninja; sets the four variables cargo needs in this session
$ErrorActionPreference = 'Stop'

$deps = $PSScriptRoot
New-Item -ItemType Directory -Force "$deps\src" | Out-Null

function Run {
	& $args[0] $args[1..($args.Count - 1)]
	if ($LASTEXITCODE -ne 0) { throw "failed: $args" }
}

function Fetch($url, $rev, $dest) {
	if (-not (Test-Path "$dest\.git")) { Run git clone -q --filter=blob:none $url $dest }
	Run git -C $dest checkout -q $rev
}

function Patch($dest, $patch) {
	git -C $dest apply --reverse --check "$deps\patches\$patch" 2>$null
	if ($LASTEXITCODE -eq 0) { return } # already applied
	Run git -C $dest apply "$deps\patches\$patch"
}

function Build($dir, [string[]]$flags) {
	Run cmake -S $dir -B "$dir\build" -G Ninja -DCMAKE_BUILD_TYPE=Release @flags
	Run cmake --build "$dir\build"
}

Fetch https://boringssl.googlesource.com/boringssl 148a7b1f361dcf6708e9eb3b7c2134d0c5633e3e "$deps\src\boringssl"
Fetch https://github.com/nghttp2/nghttp2.git v1.71.0 "$deps\src\nghttp2"
Fetch https://github.com/libuv/libuv.git v1.53.0 "$deps\src\libuv"
Fetch https://github.com/andreasbuhr/cppcoro.git 8642e98596a92be30a2b061d3ed306d959d3214e "$deps\src\cppcoro"
Fetch https://github.com/pcwalton/cxx-async.git 1575822aa7fcf484b37f43b52e60bb168b259760 "$deps\cxx-async"

Patch "$deps\src\boringssl" boringssl.patch
Patch "$deps\src\nghttp2" nghttp2.patch
Patch "$deps\cxx-async" cxx-async.patch

# no asm: no nasm needed
Build "$deps\src\boringssl" @('-DBUILD_TESTING=OFF', '-DOPENSSL_NO_ASM=ON')
Build "$deps\src\nghttp2" @('-DENABLE_LIB_ONLY=ON', '-DBUILD_SHARED_LIBS=OFF', '-DBUILD_STATIC_LIBS=ON', '-DBUILD_TESTING=OFF')
Build "$deps\src\libuv" @('-DLIBUV_BUILD_SHARED=OFF', '-DLIBUV_BUILD_TESTS=OFF')
Build "$deps\src\cppcoro" @('-DBUILD_TESTING=OFF', '-DCMAKE_CXX_STANDARD=20')

$env:BORINGSSL_DIR = "$deps\src\boringssl"
$env:NGHTTP2_DIR = "$deps\src\nghttp2"
$env:LIBUV_DIR = "$deps\src\libuv"
$env:CPPCORO_DIR = "$deps\src\cppcoro"

# ci: hand the variables to the next steps
if ($env:GITHUB_ENV) {
	foreach ($name in 'BORINGSSL_DIR', 'NGHTTP2_DIR', 'LIBUV_DIR', 'CPPCORO_DIR') {
		"$name=$((Get-Item "env:$name").Value)" | Out-File -Append -Encoding utf8 $env:GITHUB_ENV
	}
}
