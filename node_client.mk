client:
	cargo run -p fluence-client -- ${args}

LVL=warn
LVL_VERBOSE=info
DEBUG_ENV=RUST_LOG="${LVL_VERBOSE},libp2p_kad::query=${LVL_VERBOSE},fluence_server::bootstrapper=${LVL_VERBOSE},fluence_server::function::router=${LVL_VERBOSE},fluence_server::function::router_behaviour=${LVL_VERBOSE},libp2p_swarm=${LVL_VERBOSE},fluence_server=${LVL_VERBOSE},tokio_threadpool=${LVL},tokio_reactor=${LVL},mio=${LVL},tokio_io=${LVL},soketto=${LVL},yamux=${LVL},multistream_select=${LVL},libp2p_secio=${LVL},libp2p_websocket::framed=${LVL},libp2p_ping=${LVL},libp2p_core::upgrade::apply=${LVL},libp2p_kad::kbucket=${LVL},libp2p_kad::behaviour=${LVL}"

client-debug:
	${DEBUG_ENV} cargo run -p fluence-client -- ${args}

node-bootstrap:
	RUST_BACKTRACE=1 ${DEBUG_ENV} \
	cargo run -p particle-server -- -t 7770 -w 9990 \
	-k wCUPkGaBypwbeuUnmgVyN37j9iavRoqzAkddDUzx3Yir7q1yuTp3H8cdUZERYxeQ8PEiMYcDuiy1DDkfueNh1Y6

node-first:
	RUST_BACKTRACE=1 ${DEBUG_ENV} \
	cargo run -p particle-server -- -t 7771 -w 9991 -b /ip4/127.0.0.1/tcp/7770 \
	-k 4pJqYfv3wXUpodE6Bi4wE8bJkpHuFbGcXrdFnT9L29j782ge7jdov7FPrbwnvwjUm4UhK5BvJvAYikCcmvCPVx9s

node-second:
	RUST_BACKTRACE=1 ${DEBUG_ENV} \
	cargo run -p particle-server -- -t 7772 -w 9992 -b /ip4/127.0.0.1/tcp/7770 \
	-k 2zgzUew3bMSgWcZ34FFS36LiJVkn3YphW2H8TDvL8JF8T4apTDxnm7GRsLppkCNGS5ytAQioxEktYq8Wr8SWAHLv

fluence-ipfs:
	RUST_BACKTRACE=1 ${DEBUG_ENV} \
	cargo run -p fluence-ipfs /ip4/127.0.0.1/tcp/7770 /dns4/ipfs2.fluence.one/tcp/5001

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
	${DEBUG_ENV} cargo run -p fluence-ipfs -- /ip4/127.0.0.1/tcp/9990/ws /dns4/ipfs1.fluence.one/tcp/5001

client-first:
	${DEBUG_ENV} cargo run -p fluence-client -- /ip4/127.0.0.1/tcp/9991/ws

client-second:
	${DEBUG_ENV} cargo run -p fluence-client -- /ip4/127.0.0.1/tcp/9992/ws

client-tmux:
	tmux \
	new-session  'make client-first' \; \
	split-window 'sleep 1 && make client-second' \; \
	split-window 'sleep 1 && make client-ipfs' \; \
	select-layout tiled

RUST_ENV=RUST_BACKTRACE=1 RUST_LOG="info,libp2p_kad=trace,fluence_server=trace"
FLUENCE=${RUST_ENV} ./target/debug/particle-server
BOOTSTRAP=/ip4/127.0.0.1/tcp/7770
node-many:
	cargo update -p libp2p
	cargo build
	tmux \
	new-session '${FLUENCE} -d ./.fluence/7770 -t 7770 -w 9990 -k wCUPkGaBypwbeuUnmgVyN37j9iavRoqzAkddDUzx3Yir7q1yuTp3H8cdUZERYxeQ8PEiMYcDuiy1DDkfueNh1Y6' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${FLUENCE} -d ./.fluence/7771 -t 7771 -w 9991 -b ${BOOTSTRAP} -k 4pJqYfv3wXUpodE6Bi4wE8bJkpHuFbGcXrdFnT9L29j782ge7jdov7FPrbwnvwjUm4UhK5BvJvAYikCcmvCPVx9s' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${FLUENCE} -d ./.fluence/7772 -t 7772 -w 9992 -b ${BOOTSTRAP} -k 2zgzUew3bMSgWcZ34FFS36LiJVkn3YphW2H8TDvL8JF8T4apTDxnm7GRsLppkCNGS5ytAQioxEktYq8Wr8SWAHLv' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${FLUENCE} -d ./.fluence/7773 -t 7773 -w 9993 -b ${BOOTSTRAP} -k 52WaZJDHFFZbwL177g497ctE7zqbMYMwWpVMewjc1U63tWjFUCNPuzB472UkdZWBykjiNWA8qtLYNAQEqQCcWfoP' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${FLUENCE} -d ./.fluence/7774 -t 7774 -w 9994 -b ${BOOTSTRAP} -k 23BFr8LKiiAtULuYJTmLGxqDVHnjFCzNFTZcKq6g82H9kcTNwGq8Axkdow4fh4u4w763jF6uYVK6FuGESAQBMEPB' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${FLUENCE} -d ./.fluence/7775 -t 7775 -w 9995 -b ${BOOTSTRAP} -k 3wR6FT1ZGnEwPqYBNz5YVpA6qJ4uUTLcK1SpWrwJennH5Bk4JgCjKKjUiRcjDk3Cwjbm2LAdrLWTYXHjxbogttQ9' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${FLUENCE} -d ./.fluence/7776 -t 7776 -w 9996 -b ${BOOTSTRAP} -k 3KoMfcGUox46Brcnojs8yuNZN2YTH7kvmxW8g5PiRrDE2dCiQeZzhDkaJvmDDnUaHFRp6UvdmBsDrYWywYoNDqHD' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${FLUENCE} -d ./.fluence/7777 -t 7777 -w 9997 -b ${BOOTSTRAP} -k 5yqQfXyjMGKZaykByyrEjCYqxjHaCxQKxLQ3vfzU4M8U51auCiCeKT5UvnZGgMFbwwrjePUMYPvThyXiimGvq16x' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${FLUENCE} -d ./.fluence/7778 -t 7778 -w 9998 -b ${BOOTSTRAP} -k g59HxPYa1gxDZbMEtt2sry9tncQ1XwJMqoYh47JVsXnXeTf7svtzdag2pbXr6uN35L43nmKN2W4XW9MX7g5AUPB' \; \
	select-layout tiled \; \
	split-window 'sleep 1 && ${FLUENCE} -d ./.fluence/7779 -t 7779 -w 9999 -b ${BOOTSTRAP} -k 6mFRjb32PY4eMimBJq6sS6L2yW69ShHk46KTycP6v4unfBF7JYQc2Z8m8i4RPr6snWeH7Mq4ae7wCQLqV2xuCrv' \; \
	setw synchronize-panes \; \
	select-layout tiled

CLIENT=${DEBUG_ENV} cargo run -p fluence-client --
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

COUNT=15
ULIMIT=ulimit -n 10000 &&
INFO_ENV=${DEBUG_ENV}
TEST_ENV=${INFO_ENV} HOST=127.0.0.1 ASYNC_STD_THREAD_COUNT=16 RUST_BACKTRACE=full COUNT=${COUNT}
MULTIPLE_NODES=${TEST_ENV} cargo test --release --test run_multiple_nodes main -- --nocapture | tee -a "$(date)_node.log"

massive-node:
	tmux \
    	new-session  '${ULIMIT} PORT=20000 ${MULTIPLE_NODES}' \; \
    	select-layout tiled \; \
    	split-window 'sleep 1 && ${ULIMIT} PORT=30000 BS_PORT=20000 ${MULTIPLE_NODES}' \; \
    	select-layout tiled \; \
    	split-window 'sleep 1 && ${ULIMIT} PORT=40000 BS_PORT=30000 ${MULTIPLE_NODES}' \; \
    	select-layout tiled

RND=`printf '%02d' $$(( RANDOM % ${COUNT} ))`
massive-clients:
	tmux \
    	new-session  '${CLIENT} /ip4/127.0.0.1/tcp/200${RND}' \; \
    	select-layout tiled \; \
    	split-window 'sleep 1 && ${CLIENT} /ip4/127.0.0.1/tcp/300${RND}' \; \
    	select-layout tiled \; \
    	split-window 'sleep 1 && ${CLIENT} /ip4/127.0.0.1/tcp/400${RND}' \; \
    	select-layout tiled

.PHONY: client client-debug client-tmux node-tmux client-many node-many try
