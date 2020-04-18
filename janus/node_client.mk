client:
	cargo run -p janus-client -- ${args}

DEBUG_ENV=RUST_LOG="debug,tokio_threadpool=info,tokio_reactor=info,mio=info,tokio_io=info,soketto=info,yamux=info,multistream_select=info,libp2p_secio=info,libp2p_websocket::framed=info,libp2p_ping=info,libp2p_core::upgrade::apply=info"

client-debug:
	${DEBUG_ENV} cargo run -p janus-client -- ${args}

node-bootstrap:
	RUST_BACKTRACE=1 ${DEBUG_ENV} \
	cargo run -p janus-server -- -n 7770 -o 9990 \
	-k wCUPkGaBypwbeuUnmgVyN37j9iavRoqzAkddDUzx3Yir7q1yuTp3H8cdUZERYxeQ8PEiMYcDuiy1DDkfueNh1Y6

node-first:
	RUST_BACKTRACE=1 ${DEBUG_ENV} \
	cargo run -p janus-server -- -n 7771 -o 9991 -b /ip4/127.0.0.1/tcp/7770 \
	-k 4pJqYfv3wXUpodE6Bi4wE8bJkpHuFbGcXrdFnT9L29j782ge7jdov7FPrbwnvwjUm4UhK5BvJvAYikCcmvCPVx9s

node-second:
	RUST_BACKTRACE=1 ${DEBUG_ENV} \
	cargo run -p janus-server -- -n 7772 -o 9992 -b /ip4/127.0.0.1/tcp/7770 \
	-k 2zgzUew3bMSgWcZ34FFS36LiJVkn3YphW2H8TDvL8JF8T4apTDxnm7GRsLppkCNGS5ytAQioxEktYq8Wr8SWAHLv

node-tmux:
	cargo update -p libp2p
	cargo build
	tmux \
	new-session  'make node-bootstrap' \; \
	split-window 'sleep 1 && make node-first' \; \
	split-window 'sleep 2 && make node-second' \; \
	setw synchronize-panes \; \
	select-layout tiled

client-ipfs:
	${DEBUG_ENV} cargo run -p janus-ipfs -- /ip4/127.0.0.1/tcp/9990/ws /dns4/ipfs1.fluence.one/tcp/5001

client-first:
	${DEBUG_ENV} cargo run -p janus-client -- /ip4/104.248.25.59/tcp/9001/ws

client-second:
	${DEBUG_ENV} cargo run -p janus-client -- /ip4/104.248.25.59/tcp/9002/ws

client-tmux:
	tmux \
	new-session  'make client-first' \; \
	split-window 'sleep 1 && make client-second' \; \
	select-layout tiled

RUST_ENV=RUST_BACKTRACE=1 RUST_LOG="info,libp2p_kad=trace,janus_server=trace"
JANUS=${RUST_ENV} ./target/debug/janus-server
BOOTSTRAP=/ip4/127.0.0.1/tcp/7770
node-many:
	cargo update -p libp2p
	cargo build
	tmux \
	new-session '${JANUS} -d ./.janus/7770 -n 7770 -o 9990 -k wCUPkGaBypwbeuUnmgVyN37j9iavRoqzAkddDUzx3Yir7q1yuTp3H8cdUZERYxeQ8PEiMYcDuiy1DDkfueNh1Y6' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${JANUS} -d ./.janus/7771 -n 7771 -o 9991 -b ${BOOTSTRAP} -k 4pJqYfv3wXUpodE6Bi4wE8bJkpHuFbGcXrdFnT9L29j782ge7jdov7FPrbwnvwjUm4UhK5BvJvAYikCcmvCPVx9s' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${JANUS} -d ./.janus/7772 -n 7772 -o 9992 -b ${BOOTSTRAP} -k 2zgzUew3bMSgWcZ34FFS36LiJVkn3YphW2H8TDvL8JF8T4apTDxnm7GRsLppkCNGS5ytAQioxEktYq8Wr8SWAHLv' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${JANUS} -d ./.janus/7773 -n 7773 -o 9993 -b ${BOOTSTRAP} -k 52WaZJDHFFZbwL177g497ctE7zqbMYMwWpVMewjc1U63tWjFUCNPuzB472UkdZWBykjiNWA8qtLYNAQEqQCcWfoP' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${JANUS} -d ./.janus/7774 -n 7774 -o 9994 -b ${BOOTSTRAP} -k 23BFr8LKiiAtULuYJTmLGxqDVHnjFCzNFTZcKq6g82H9kcTNwGq8Axkdow4fh4u4w763jF6uYVK6FuGESAQBMEPB' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${JANUS} -d ./.janus/7775 -n 7775 -o 9995 -b ${BOOTSTRAP} -k 3wR6FT1ZGnEwPqYBNz5YVpA6qJ4uUTLcK1SpWrwJennH5Bk4JgCjKKjUiRcjDk3Cwjbm2LAdrLWTYXHjxbogttQ9' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${JANUS} -d ./.janus/7776 -n 7776 -o 9996 -b ${BOOTSTRAP} -k 3KoMfcGUox46Brcnojs8yuNZN2YTH7kvmxW8g5PiRrDE2dCiQeZzhDkaJvmDDnUaHFRp6UvdmBsDrYWywYoNDqHD' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${JANUS} -d ./.janus/7777 -n 7777 -o 9997 -b ${BOOTSTRAP} -k 5yqQfXyjMGKZaykByyrEjCYqxjHaCxQKxLQ3vfzU4M8U51auCiCeKT5UvnZGgMFbwwrjePUMYPvThyXiimGvq16x' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${JANUS} -d ./.janus/7778 -n 7778 -o 9998 -b ${BOOTSTRAP} -k g59HxPYa1gxDZbMEtt2sry9tncQ1XwJMqoYh47JVsXnXeTf7svtzdag2pbXr6uN35L43nmKN2W4XW9MX7g5AUPB' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${JANUS} -d ./.janus/7779 -n 7779 -o 9999 -b ${BOOTSTRAP} -k 6mFRjb32PY4eMimBJq6sS6L2yW69ShHk46KTycP6v4unfBF7JYQc2Z8m8i4RPr6snWeH7Mq4ae7wCQLqV2xuCrv' \; \
	setw synchronize-panes \; \
	select-layout tiled

CLIENT=${DEBUG_ENV} cargo run -p janus-client --
client-many:
	tmux \
	new-session  '${CLIENT} /ip4/127.0.0.1/tcp/9990/ws' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${CLIENT} /ip4/127.0.0.1/tcp/9991/ws' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${CLIENT} /ip4/127.0.0.1/tcp/9992/ws' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${CLIENT} /ip4/127.0.0.1/tcp/9993/ws' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${CLIENT} /ip4/127.0.0.1/tcp/9994/ws' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${CLIENT} /ip4/127.0.0.1/tcp/9995/ws' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${CLIENT} /ip4/127.0.0.1/tcp/9996/ws' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${CLIENT} /ip4/127.0.0.1/tcp/9997/ws' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${CLIENT} /ip4/127.0.0.1/tcp/9998/ws' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${CLIENT} /ip4/127.0.0.1/tcp/9999/ws' \; \
	select-layout tiled

.PHONY: client client-debug client-tmux node-tmux client-many node-many
