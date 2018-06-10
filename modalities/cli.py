#!/usr/bin/env python3

import json
import socket
import struct
import threading
import queue

# log = logger.create('cli.log')
# log.setLevel(logger.logging.INFO)

def dispatch(msg, queue):
    print(msg)
    return True

# TODO: Experiment with creating a plugin architecture (and loading the cli app in through that)
    # Work on the plugin architecture inside of `client.py'`
# TODO: Work on `client.py` package to provie a usable interface
# TODO: Need to add in a console lock to interleave input/output
    # This lock should be "breakable" after a little while of non-use
# TODO: Closing the server should close this app
    # I think I'm actually going to send an explicit "quit" command though
    # Would need to have a wait to stop the cli loop though



# TODO: Look at using select to implement this "structure" (instead of threading)
# TODO: Implement this framework with the plugin architecture

def get_messages(socket):
    try:
        while True:
            len_buf = socket.recv(4)
            msg_len = struct.unpack(">I", len_buf)[0]
            buf = socket.recv(msg_len)
            yield json.loads(buf.decode('utf-8'))

    except ConnectionResetError:
        print("Connection to server lost")

    finally:
        return

def send_message(socket, msg):
    data = json.dumps(msg).encode('utf-8')
    frame = struct.pack(">I", len(data))
    socket.sendall(frame + data)

def reader(socket, queue):
    for msg in get_messages(socket):
        dispatch(msg, queue)
    queue.put("quit")

def writer(socket, queue):
    while True:
        msg = queue.get()
        if msg == "quit": break
        send_message(socket, msg)

# TODO: Add way to "quit" this function when the app closes (maybe make this the threaded func?)
def run_main(queue):
    while True:
        query = input("> ")
        if query == "quit": break
        queue.put({ 'msg': query })

    queue.put("quit")

if __name__ == "__main__":
    queue = queue.Queue()
    sock = socket.socket()
    sock.connect(('127.0.0.1', 6142))

    read_thread = threading.Thread(target=reader, args=(sock, queue,))
    write_thread = threading.Thread(target=writer, args=(sock, queue,))

    write_thread.start()
    read_thread.start()

    queue.put({ 'msg': 'hello' })
    run_main(queue)

    write_thread.join()
    read_thread.join()
    sock.close()

# API Documentation:
#   Pyro4: https://pythonhosted.org/Pyro4/
