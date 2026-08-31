echo "This is loud stdout output that must not pollute posix-source stdout"
echo "This is loud stderr output" >&2
export NOISY_VAR="success"
export LOUD_SETTING="1"
