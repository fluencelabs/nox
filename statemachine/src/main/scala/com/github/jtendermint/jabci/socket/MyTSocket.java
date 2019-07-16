/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

package com.github.jtendermint.jabci.socket;

/*
 * The MIT License (MIT)
 *
 * Copyright (c) 2016 - 2018
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */

import java.io.IOException;
import java.net.ServerSocket;
import java.net.Socket;
import java.net.SocketTimeoutException;
import java.util.Collections;
import java.util.Objects;
import java.util.Optional;
import java.util.Set;
import java.util.concurrent.ConcurrentHashMap;

import com.google.protobuf.CodedOutputStream;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

import com.github.jtendermint.jabci.socket.ExceptionListener.Event;
import com.github.jtendermint.jabci.types.Request;
import com.github.jtendermint.jabci.types.ResponseException;
import com.google.protobuf.CodedInputStream;
import com.google.protobuf.GeneratedMessageV3;
import com.google.protobuf.InvalidProtocolBufferException;

/**
 * The TSocket acts as the main socket connection to the Tendermint-Node
 *
 * @author srmo, wolfposd
 */
public class MyTSocket extends ASocket {

    public static final int DEFAULT_LISTEN_SOCKET_PORT = 26658;
    public static final String INFO_SOCKET = "-Info";
    public static final String MEMPOOL_SOCKET = "-MemPool";
    public static final String CONSENSUS_SOCKET = "-Consensus";
    public static final int DEFAULT_LISTEN_SOCKET_TIMEOUT = 1000;

//    private static final Logger System.out.format()TSocket: .getLogger(MyTSocket.class + "\n);
//    private static final Logger System.out.format.gTSocket: etLogger(SocketHandler.class + "\n);

    private final Set<SocketHandler> runningThreads = Collections.newSetFromMap(new ConcurrentHashMap<SocketHandler, Boolean>());
    private long lastConnectedSocketTime = -1;
    private boolean continueRunning = true;
    private ExceptionListener exceptionListener;
    private ConnectionListener connectionListener;
    private DisconnectListener disconnectListener;

    public MyTSocket() {
        this((socket, cause, exception) -> {
        }, (name, count) -> {
        }, (name, count) -> {
        });
    }

    /**
     * Creates this MyTSocket with the ability to register for exceptions that occur
     * and listen to new socket connections
     *
     * @param exceptionListener
     *            listens to exceptions
     * @param connectionListener
     *            listens to new connections
     * @param disconnectListener
     *            listens to disconnects
     */
    public MyTSocket(ExceptionListener exceptionListener, ConnectionListener connectionListener, DisconnectListener disconnectListener) {
        this.connectionListener = Objects.requireNonNull(connectionListener, "requires a connectionListener");
        this.exceptionListener = Objects.requireNonNull(exceptionListener, "requires an exceptionListener");
        this.disconnectListener = Objects.requireNonNull(disconnectListener, "requires a disconnectListener");
    }

    /**
     * Start listening on the default ABCI port 46658
     */
    public void start() {
        this.start(DEFAULT_LISTEN_SOCKET_PORT, DEFAULT_LISTEN_SOCKET_TIMEOUT);
    }

    /**
     * Start listening on the specified port
     *
     * @param portNumber tendermint abci port
     */
    public void start(final int portNumber) {
        this.start(portNumber, DEFAULT_LISTEN_SOCKET_TIMEOUT);
    }

    /**
     * Start listening on the specified port
     *
     * @param portNumber
     * @param socketTimeout
     */
    public void start(final int portNumber, final int socketTimeout) {
        System.out.format("TSocket: starting serversocket" + "\n");
        continueRunning = true;
        int socketcount = 0;
        try (ServerSocket serverSocket = new ServerSocket(portNumber)) {
            serverSocket.setSoTimeout(socketTimeout);
            while (continueRunning) {
                try {
                    Socket clientSocket = serverSocket.accept();
                    lastConnectedSocketTime = System.currentTimeMillis();
                    String socketName = socketNameForCount(++socketcount);
                    System.out.format("TSocket: starting socket with" + socketName + "\n");
                    SocketHandler t = (socketName != null) ? new SocketHandler(clientSocket, socketName) : new SocketHandler(clientSocket);
                    t.start();
                    runningThreads.add(t);
                    System.out.format("TSocket: Started thread for sockethandling..." + "\n");
                    connectionListener.connected(Optional.ofNullable(socketName), runningThreads.size());
                } catch (SocketTimeoutException ste) {
                    // this is triggered by accept()
                    exceptionListener.notifyExceptionOccured(Optional.ofNullable(socketNameForCount(socketcount)), Event.Socket_accept, ste);
                }
            }
            System.out.format("TSocket: MyTSocket Stopped Running" + "\n");
        } catch (IOException e) {
            exceptionListener.notifyExceptionOccured(Optional.ofNullable(socketNameForCount(socketcount)), Event.ServerSocket, e);
            System.out.format("TSocket: Exception caught when trying to listen on port " + portNumber + " or listening for a connection", e + "\n");
        }
        System.out.format("TSocket: Exited main-run-while loop" + "\n");
    }

    private String socketNameForCount(int c) {
        switch (c) {
            case 1:
                return INFO_SOCKET;
            case 2:
                return MEMPOOL_SOCKET;
            case 3:
                return CONSENSUS_SOCKET;
            default:
                return null;
        }
    }

    public void stop() {
        continueRunning = false;
        runningThreads.forEach(t -> t.interrupt());

        try {
            // wait two seconds to finalize last messages on streams
            Thread.sleep(2000);
        } catch (InterruptedException e) {
            exceptionListener.notifyExceptionOccured(Optional.empty(), Event.Thread_sleep, e);
        }

        // close socket connections
        runningThreads.forEach(t -> {
            t.closeConnections();
        });

        runningThreads.clear();
        Thread.currentThread().interrupt();
        System.out.format("TSocket: Finished calling stop on members." + "\n");
    }

    /**
     * @return the amount of connected sockets, this should usually be 3: info,mempool and consensus
     */
    public int sizeOfConnectedABCISockets() {
        return runningThreads.size();
    }

    public long getLastConnectedTime() {
        return lastConnectedSocketTime;
    }

    class SocketHandler extends Thread {

        private final Socket socket;
        private CodedInputStream inputStream;
        private CodedOutputStream outputStream;
        private boolean nameSet = false;

        public SocketHandler(Socket socket) {
            this.setDaemon(true);
            this.socket = socket;
            this.updateName("" + socket.getPort());
        }

        public SocketHandler(Socket socket, String name) {
            this.setDaemon(true);
            this.socket = socket;
            nameSet = true;
            this.updateName(name);
        }

        public void updateName(String name) {
            this.setName("SocketThread" + name);
        }

        @Override
        public void interrupt() {
            System.out.format("TSocket: " + this.getName() + "being interrupted" + "\n");
            super.interrupt();
        }

        private void closeConnections() {
            try {
                if (socket != null) {
                    System.out.format("TSocket: {} outputStream close", getName() + "\n");
                    if (!socket.isClosed())
                        socket.getOutputStream().close();

                    System.out.format("TSocket: {} inputStream close", getName() + "\n");
                    if (!socket.isClosed())
                        socket.getInputStream().close();

                    System.out.format("TSocket: {} socket close", getName() + "\n");
                    socket.close();
                }
            } catch (Exception e) {
                exceptionListener.notifyExceptionOccured(Optional.ofNullable(this.getName()), Event.SocketHandler_closeConnections, e);
            }
            runningThreads.remove(this);
        }

        @Override
        public void run() {

            System.out.format("TSocket: Starting ThreadNo: " + getName() + "\n");
            System.out.format("TSocket: accepting new client" + "\n");
            try {
                inputStream = CodedInputStream.newInstance(socket.getInputStream());
                outputStream = CodedOutputStream.newInstance(socket.getOutputStream());
                while (!isInterrupted() && !socket.isClosed()) {
//                    System.out.format("TSocket: start reading" + "\n");

                    // Size counter is used to enforce a size limit per message (see CodedInputStream.setSizeLimit()).
                    // We need to reset it before reading the next message:
                    inputStream.resetSizeCounter();

                    // HEADER: first byte(s) is length of the following message;
                    // it is protobuf encoded as a varint-uint64
                    try {
                        int varintLengthByte = (int) CodedInputStream.decodeZigZag64(inputStream.readUInt64());

                        int oldLimit = inputStream.pushLimit(varintLengthByte);
                        final Request request = Request.parseFrom(inputStream);
                        inputStream.popLimit(oldLimit);

                        if (!nameSet) {
                            determineSocketNameAndUpdate(request.getValueCase());
                        }

                        // Process the request that was just read:
                        try {
                            GeneratedMessageV3 response = handleRequest(request);
                            writeMessage(response);
                        } catch (Exception e) {
                            exceptionListener.notifyExceptionOccured(Optional.ofNullable(this.getName()), Event.SocketHandler_handleRequest, e);
                            ResponseException exception = ResponseException.newBuilder().setError(e.getMessage()).build();
                            writeMessage(exception);
                        }

                    } catch (InvalidProtocolBufferException ipbe) {
                        this.interrupt();
                        this.socket.close();
                        exceptionListener.notifyExceptionOccured(Optional.ofNullable(this.getName()), Event.SocketHandler_readFromStream, ipbe);
                    }
                }
            } catch (IOException e) {
                exceptionListener.notifyExceptionOccured(Optional.ofNullable(this.getName()), Event.SocketHandler_run, e);
                if (!isInterrupted()) {
                    System.out.format("TSocket: Error with " + this.getName(), e + "\n");
                    System.out.format("TSocket: Note: If \"the input ended unexpectedly\" it could mean: \n - tendermint was shut down\n - the protobuf file is not up to date." + "\n");
                }
            }
            System.out.format("TSocket: Stopping Thread " + this.getName() + "\n");
            Thread.currentThread().interrupt();
            runningThreads.remove(this);
            disconnectListener.disconnected(Optional.ofNullable(this.getName()), runningThreads.size());
        }

        private void determineSocketNameAndUpdate(Request.ValueCase valuecase) {
            switch (valuecase) {
                case FLUSH:
                    break;
                case INFO:
                    this.updateName(INFO_SOCKET);
                    nameSet = true;
                    break;
                case CHECK_TX:
                    this.updateName(MEMPOOL_SOCKET);
                    nameSet = true;
                    break;
                default:
                    this.updateName(CONSENSUS_SOCKET);
                    nameSet = true;
                    break;
            }
        }

        /**
         * Writes a {@link GeneratedMessageV3} to the socket output stream
         *
         * @param message
         * @throws IOException
         */
        public void writeMessage(GeneratedMessageV3 message) throws IOException {
            if (message != null) {
                System.out.format("TSocket: writing message " + message.getAllFields().keySet() + "\n");
                long length = message.getSerializedSize();

                // HEADER: first byte(s) is varint encoded length of the message
                // also, see writeMessage for the other way round
                outputStream.writeUInt64NoTag(CodedOutputStream.encodeZigZag64(length));
                message.writeTo(outputStream);
                outputStream.flush();
            }
        }
    }

}
