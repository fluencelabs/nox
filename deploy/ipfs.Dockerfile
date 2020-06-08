FROM ipfs/go-ipfs:release

COPY ./ipfs /usr/local/bin/ipfs
RUN chown ipfs:users /usr/local/bin/ipfs
RUN chmod +x /usr/local/bin/ipfs

# # Swarm TCP; should be exposed to the public
# EXPOSE 4001
# # Daemon API; must not be exposed publicly but to client services under you control
# EXPOSE 5001
# # Web Gateway; can be exposed publicly with a proxy, e.g. as https://ipfs.example.org
# EXPOSE 8080
# # Swarm Websockets; must be exposed publicly when the node is listening using the websocket transport (/ipX/.../tcp/8081/ws).
# EXPOSE 8081

# Create the fs-repo directory and switch to a non-privileged user.
ENV IPFS_PATH /data/ipfs
RUN mkdir -p $IPFS_PATH \
  && chown ipfs:users $IPFS_PATH

# Create mount points for `ipfs mount` command
RUN mkdir -p /ipfs /ipns \
  && chown ipfs:users /ipfs /ipns

# Expose the fs-repo as a volume.
# start_ipfs initializes an fs-repo if none is mounted.
# Important this happens after the USER directive so permission are correct.
VOLUME $IPFS_PATH

# The default logging level
ENV IPFS_LOGGING ""

# This just makes sure that:
# 1. There's an fs-repo, and initializes one if there isn't.
# 2. The API and Gateway are accessible from outside the container.
ENTRYPOINT ["/sbin/tini", "--", "/usr/local/bin/start_ipfs"]

# Execute the daemon subcommand by default
CMD ["daemon", "--migrate=true"]
