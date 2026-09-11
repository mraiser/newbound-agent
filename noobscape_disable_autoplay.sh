#!/bin/sh
set -e
BIN=/noobscape/bin/firefox
# resolve the symlink so the files land beside the REAL binary
# (the builder's mode=symlink install points into the build's dist/bin)
DIR=$(dirname "$(readlink -f "$BIN")")

mkdir -p "$DIR/defaults/pref"

cat > "$DIR/defaults/pref/autoconfig.js" <<'EOF'
pref("general.config.filename", "noobscape.cfg");
pref("general.config.obscure_value", 0);
EOF

cat > "$DIR/noobscape.cfg" <<'EOF'
// Noobscape autoconfig — first line must be a comment
lockPref("media.autoplay.default", 5);              // 0=allow, 1=block audio, 5=block audio+video
lockPref("media.autoplay.blocking_policy", 2);      // never autoplay, even after user interaction
lockPref("media.autoplay.allow-extension-background-pages", false);
lockPref("media.autoplay.block-event.enabled", true);
EOF
