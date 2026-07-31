#!/bin/bash
#
# Nexus CLI installer — https://github.com/Alele496/Nexus/releases
#
# Downloads the latest Nexus binary from GitHub Releases and installs to ~/.nexus/bin/
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/Alele496/Nexus/agent-dev/nexus/crates/codegen/nexus-pager/scripts/install.sh | bash
#   curl -fsSL <url> | bash -s 0.3.0  # specific version
#
# Windows: run under Git for Windows / MSYS2 Bash (same curl | bash flow);
# WSL uses the Linux binary.

set -e

TARGET="$1"

if [[ -n "$TARGET" ]] && [[ ! "$TARGET" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9._]+)?$ ]]; then
    echo "Invalid version format: $TARGET (expected X.Y.Z or X.Y.Z-suffix)" >&2
    exit 1
fi

DOWNLOADER=""
if command -v curl >/dev/null 2>&1; then
    DOWNLOADER="curl"
elif command -v wget >/dev/null 2>&1; then
    DOWNLOADER="wget"
else
    echo "Either curl or wget is required but neither is installed" >&2
    exit 1
fi

download_file() {
    local url="$1" output="$2"
    if [ "$DOWNLOADER" = "curl" ]; then
        if [ -n "$output" ]; then
            curl -fsSL -o "$output" "$url"
        else
            curl -fsSL "$url"
        fi
    else
        if [ -n "$output" ]; then
            wget -q -O "$output" "$url"
        else
            wget -q -O - "$url"
        fi
    fi
}

download_file_parallel() {
    local url="$1" output="$2"
    if [ "$DOWNLOADER" != "curl" ]; then
        download_file "$url" "$output"
        return
    fi
    local size
    size=$(curl -fsSL --head "$url" 2>/dev/null | awk -F'[: \r\n]+' 'tolower($1)=="content-length"{print $2; exit}')
    if [ -z "$size" ] || ! [ "$size" -ge 16777216 ] 2>/dev/null; then
        download_file "$url" "$output"
        return
    fi
    local n=8
    local chunk_size=$(( (size + n - 1) / n ))
    local tmpdir
    tmpdir=$(mktemp -d 2>/dev/null) || { download_file "$url" "$output"; return; }
    local pids=() i start end
    for i in $(seq 0 $((n - 1))); do
        start=$((i * chunk_size))
        end=$((start + chunk_size - 1))
        [ $end -ge $size ] && end=$((size - 1))
        curl -fsSL -r "${start}-${end}" -o "${tmpdir}/$(printf 'chunk.%03d' "$i")" "$url" &
        pids+=($!)
    done
    local all_ok=true pid
    for pid in "${pids[@]}"; do
        wait "$pid" || all_ok=false
    done
    if [ "$all_ok" = true ] && cat "${tmpdir}"/chunk.* > "$output" 2>/dev/null; then
        rm -rf "$tmpdir"
        return 0
    fi
    rm -rf "$tmpdir"
    download_file "$url" "$output"
}

# ── Detect OS and architecture ─────────────────────────────────────────────

case "$(uname -s)" in
    Darwin) os="macos" ;;
    Linux)  os="linux" ;;
    MINGW* | MSYS* | CYGWIN*) os="windows" ;;
    *)      echo "Unsupported OS: $(uname -s)" >&2; exit 1 ;;
esac

case "$(uname -m)" in
    x86_64|amd64|AMD64) arch="x86_64" ;;
    arm64|aarch64|ARM64) arch="aarch64" ;;
    *)                    echo "Unsupported architecture: $(uname -m)" >&2; exit 1 ;;
esac

NEXUS_DIR="${NEXUS_HOME:-$HOME/.nexus}"
DOWNLOAD_DIR="$NEXUS_DIR/downloads"
BIN_DIR="${NEXUS_BIN_DIR:-$NEXUS_DIR/bin}"
mkdir -p "$DOWNLOAD_DIR" "$BIN_DIR"

platform="${os}-${arch}"

GITHUB_RELEASES="https://github.com/Alele496/Nexus/releases"

# ── Resolve version ────────────────────────────────────────────────────────

if [ -z "$TARGET" ]; then
    echo "Fetching latest Nexus version..." >&2
    version=$(download_file "https://api.github.com/repos/Alele496/Nexus/releases/latest" 2>/dev/null | \
        sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\(v[^"]*\)".*/\1/p' | head -1 | tr -d 'v')
    if [ -z "$version" ]; then
        echo "Error: failed to fetch latest version from GitHub." >&2
        echo "Try specifying a version: curl -fsSL <url> | bash -s 0.3.0" >&2
        exit 1
    fi
else
    version="$TARGET"
fi

if [[ ! "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+(-[A-Za-z0-9._]+)?$ ]]; then
    echo "Invalid version format: $version (expected X.Y.Z or X.Y.Z-suffix)" >&2
    exit 1
fi

echo "Installing Nexus v$version ($platform)..." >&2

# ── Download binary ────────────────────────────────────────────────────────

artifact_name="nexus-${version}-${platform}"
artifact_url="${GITHUB_RELEASES}/download/v${version}/${artifact_name}"

if [ "$os" = "windows" ]; then
    artifact_name="${artifact_name}.exe"
    artifact_url="${artifact_url}.exe"
fi

binary_path="$DOWNLOAD_DIR/$artifact_name"
binary_tmp="${binary_path}.tmp.$$"
rm -f "$binary_tmp" 2>/dev/null || true

echo "  Downloading nexus v${version}..." >&2
if ! download_file_parallel "$artifact_url" "$binary_tmp"; then
    rm -f "$binary_tmp"
    echo "Error: binary download failed from ${artifact_url}" >&2
    exit 1
fi

if [ "$os" = "windows" ]; then
    mv -f "$binary_tmp" "$binary_path"
    rm -f "$BIN_DIR/nexus.old" 2>/dev/null || true
    if ! cp -f "$binary_path" "$BIN_DIR/nexus.exe" 2>/dev/null; then
        mv -f "$BIN_DIR/nexus.exe" "$BIN_DIR/nexus.old" 2>/dev/null || true
        if ! cp -f "$binary_path" "$BIN_DIR/nexus.exe" 2>/dev/null; then
            mv -f "$BIN_DIR/nexus.old" "$BIN_DIR/nexus.exe" 2>/dev/null || true
            echo "Error: failed to install nexus.exe" >&2
            exit 1
        fi
    fi
    echo "  Binary installed to $BIN_DIR/nexus.exe" >&2
else
    chmod +x "$binary_tmp"
    if ! "$binary_tmp" --version </dev/null >/dev/null 2>&1; then
        echo "Error: downloaded nexus failed to run; keeping the existing install." >&2
        rm -f "$binary_tmp"
        exit 1
    fi
    mv -f "$binary_tmp" "$binary_path"
    # Use relative symlinks when BIN_DIR and DOWNLOAD_DIR share a parent
    if [ "$(dirname "$BIN_DIR")" = "$(dirname "$DOWNLOAD_DIR")" ]; then
        link_target="../$(basename "$DOWNLOAD_DIR")/$(basename "$binary_path")"
    else
        link_target="$binary_path"
    fi
    ln -sf "$link_target" "$BIN_DIR/nexus"
    echo "  Binary linked to $BIN_DIR/nexus" >&2
fi

# ── Generate completions (best-effort) ─────────────────────────────────────

mkdir -p "$NEXUS_DIR/completions/bash" "$NEXUS_DIR/completions/zsh"
"$BIN_DIR/nexus" completions bash > "$NEXUS_DIR/completions/bash/nexus.bash" 2>/dev/null || true
"$BIN_DIR/nexus" completions zsh  > "$NEXUS_DIR/completions/zsh/_nexus"     2>/dev/null || true
if mkdir -p "$HOME/.config/fish/completions" 2>/dev/null; then
    "$BIN_DIR/nexus" completions fish > "$HOME/.config/fish/completions/nexus.fish" 2>/dev/null || true
fi

# ── Persist installer config ───────────────────────────────────────────────

CONFIG_FILE="$NEXUS_DIR/config.toml"
if [ ! -f "$CONFIG_FILE" ]; then
    printf '[cli]\ninstaller = "internal"\n' > "$CONFIG_FILE"
fi

# ── Add to PATH ────────────────────────────────────────────────────────────

path_has_dir() {
    case ":$PATH:" in *":$1:"*) return 0 ;; *) return 1 ;; esac
}

SYMLINK_CREATED=""
if [ "$os" != "windows" ] && ! path_has_dir "$BIN_DIR"; then
    for candidate in "$HOME/.local/bin" "/usr/local/bin"; do
        if path_has_dir "$candidate" && [ -d "$candidate" ] && [ -w "$candidate" ]; then
            ln -sf "$BIN_DIR/nexus" "$candidate/nexus"
            SYMLINK_CREATED="$candidate"
            echo "  Symlinked $candidate/nexus -> $BIN_DIR/nexus" >&2
            break
        fi
    done
fi

user_shell="$(basename "${SHELL:-}")"
config_file=""

case "$user_shell" in
    bash) config_file="$HOME/.bashrc" ;;
    zsh)  config_file="$HOME/.zshrc" ;;
    fish) config_file="$HOME/.config/fish/config.fish" ;;
esac

if [ -n "$config_file" ]; then
    mkdir -p "$(dirname "$config_file")"

    # Resolve symlinks so tmp+mv rewrites the actual file, not the link.
    if [ -e "$config_file" ] || [ -L "$config_file" ]; then
        _cf="$config_file"
        _depth=0
        while [ -L "$_cf" ] && [ "$_depth" -lt 40 ]; do
            _link="$(readlink "$_cf")" || break
            case "$_link" in
                /*) _cf="$_link" ;;
                *)  _cf="$(cd "$(dirname "$_cf")" && pwd -P)/$_link" ;;
            esac
            _depth=$((_depth + 1))
        done
        if [ ! -L "$_cf" ]; then
            config_file="$(cd "$(dirname "$_cf")" && pwd -P)/$(basename "$_cf")"
        fi
        unset _cf _link _depth
    fi

    if [ "$user_shell" = "fish" ]; then
        new_block='# >>> nexus installer >>>
fish_add_path $HOME/.nexus/bin
# <<< nexus installer <<<'
    elif [ "$user_shell" = "zsh" ]; then
        new_block='# >>> nexus installer >>>
export PATH="$HOME/.nexus/bin:$PATH"
fpath=(~/.nexus/completions/zsh $fpath)
autoload -Uz compinit && compinit -C
# <<< nexus installer <<<'
    else
        new_block='# >>> nexus installer >>>
export PATH="$HOME/.nexus/bin:$PATH"
[[ -r "$HOME/.nexus/completions/bash/nexus.bash" ]] && source "$HOME/.nexus/completions/bash/nexus.bash"
# <<< nexus installer <<<'
    fi

    if grep -qs "nexus installer" "$config_file" 2>/dev/null; then
        tmp="$config_file.tmp.$$"
        awk '
            /# >>> nexus installer >>>/ { skip=1; next }
            /# <<< nexus installer <<</ { skip=0; next }
            !skip { print }
        ' "$config_file" > "$tmp" && mv "$tmp" "$config_file"
    fi

    printf '\n%s\n' "$new_block" >> "$config_file"
    echo "  Added $BIN_DIR to PATH in $config_file." >&2
fi

echo "" >&2
echo "Nexus v$version installed successfully!" >&2
echo "" >&2
if path_has_dir "$BIN_DIR" || [ -n "$SYMLINK_CREATED" ]; then
    echo "Run 'nexus' to get started!" >&2
elif [ -n "$config_file" ]; then
    echo "Restart your terminal, then run 'nexus' to get started!" >&2
else
    echo "Add $BIN_DIR to your PATH, then run 'nexus' to get started:" >&2
    echo '  export PATH="$HOME/.nexus/bin:$PATH"' >&2
fi

if [ "$os" = "windows" ]; then
    echo "To use nexus from cmd.exe or PowerShell, add %USERPROFILE%\\.nexus\\bin to your PATH." >&2
fi
