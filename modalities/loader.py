#!/usr/bin/env python3

import argparse
import asyncio
import socket
import threading
import time
import traceback

from common import logger, protocol
from common import plugins as plugin_system
from common.msg import Message


def reader(comm, sock, plugin, loop):
    """
    Thread callback to dispatch and handle messages sent to this plugin
    """
    log = plugin.logger

    async def _exc_wrapper(msg):
        """
        Wrap asyncio handle to catch and report thrown exceptions
        """
        try:
            msg_handler = comm.handles.get(msg.action, plugin.handle_unknown_message)
            await msg_handler(plugin, msg, comm)

        except Exception as e:
            log.error("Exception while handling message id={}: {}".format(msg.id, e))
            log.error("  " + traceback.format_exc())

            msg.action = 'error'
            msg.args = str(e)
            msg.return_to_sender()
            comm.send(msg, log)

    # For every message that we receive from the server
    for msg in protocol.JsonCodec.get_messages(sock, log):
        if Message.is_quit(msg):
            log.debug("Received quit message in reader thread: msg.id={}".format(msg.id))
            break

        # If we have requested this message in some other handler
        elif msg.id in comm.events:
            log.info("Received response to message id={}. Resuming sender routine".format(msg.id))

            comm.events[msg.id].value = msg
            loop.call_soon_threadsafe(comm.events[msg.id].set)

        # Otherwise call the registered plugin handler
        else:
            log.info("Handling message id={} through plugin handles: action={}".format(msg.id, msg.action))
            asyncio.run_coroutine_threadsafe(_exc_wrapper(msg), loop=loop)

    log.debug("Closing reader thread")


def writer(comm, sock, log):
    """
    Thread callback responsible for sending messages out of the plugin
    """
    while True:
        msg = comm.get_msg()
        protocol.JsonCodec.send_message(msg, sock, log)

        if Message.is_quit(msg):
            log.debug("Received quit message in writer thread")
            break


async def handshake(plugin, _plugin_handles, comm):
    plugin.logger.info("Initiating plugin handshake with device-manager")

    msg = Message(plugin=plugin)
    msg.action = 'handshake'
    msg.send_to(role='manager')
    await comm.wait_for_response(msg, plugin.logger)

    plugin.logger.info("Completed plugin handshake with device-manager")


async def run(plugin, comm, read_thread, write_thread):
    """
    Run the plugin within the asyncio event loop
    """
    log = plugin.logger

    try:
        while True:
            finish_run = await plugin.run(comm)

            if not finish_run:
                log.info("Stopping plugin because plugin finished running")
                break
            if not write_thread.is_alive():
                log.debug("Stopping plugin because writer thread has stopped")
                break
            if not read_thread.is_alive():
                log.debug("Stopping plugin because reader thread has stopped")
                break

            await asyncio.sleep(5)

    except:
        log.error("EXCEPTION: " + traceback.format_exc())

    log.debug("Stopped running main plugin run loop. Sending stop message to device-manager")

    msg = Message(plugin=plugin)
    msg.action = Message.STOP
    msg.send_to(role='manager')
    comm.send(msg, log)


if __name__ == "__main__":
    # Parse the command line for the loader arguments
    parser = argparse.ArgumentParser(description='Load {personal ai} plugin')
    parser.add_argument('--plugin-dir', type=str, help='location of plugins')
    # NOTE: Because the plugins are device-local, the host is almost guaranteed to always be `localhost`. However, I will keep the configuration just in case
    parser.add_argument('--host', type=str, default='127.0.0.1', help='ip address of the host server')
    parser.add_argument('--port', type=int, help='port that the server is listening on')
    parser.add_argument('--log-dir', type=str, help='location to write log files')
    parser.add_argument('--log-level', type=str, help='logging level', default='INFO')
    parser.add_argument('--retry-delay', type=int, help='Num seconds to sleep in between connection retries')
    parser.add_argument('--max-retries', type=int, help='Maximum retry attempts before connection failed')
    parser.add_argument('plugin', nargs=argparse.REMAINDER, help='Name of the plugin to launch plus all plugin-specific arguments')

    # Extract the arguments from the command line
    loader_args = vars(parser.parse_args())
    plugin_args = loader_args.pop('plugin', [])

    # Load the specified plugin
    name = plugin_args.pop(0)
    log = logger.create("loader.log", name='__loader__', log_dir=loader_args['log_dir'], level=loader_args.pop('log-level', None), fmt="%(asctime)s <%(levelname)s> [{}] %(message)s".format(name))

    plugin, handles = plugin_system.load(name, log=log, args=plugin_args, plugin_dir=loader_args['plugin_dir'], log_dir=loader_args['log_dir'])
    if plugin is None:
        log.error("Couldn't load plugin {}".format(name))
        exit(1)

    # Launch the networking threads (for communicating with the device manager)
    host, port = loader_args['host'], loader_args['port']
    sock, num_connection_attempts = socket.socket(), 0

    # Handle connection errors
    while True:
        log.debug("Attempting to connect to {}:{}".format(host, port))
        num_connection_attempts += 1

        try:
            sock.connect((host, port))
            break

        except socket.error as e:
            if num_connection_attempts == loader_args['max_retries']:
                raise e

            log.debug("Connection failed: {}".format(e))
            time.sleep(loader_args['retry_delay'])

    log.info("Connected to {}:{}".format(host, port))

    # Create the communication threads
    comm = protocol.CommChannel(handles)
    loop = asyncio.get_event_loop()
    read_thread = threading.Thread(target=reader, args=(comm, sock, plugin, loop))
    read_thread.start()
    write_thread = threading.Thread(target=writer, args=(comm, sock, plugin.logger))
    write_thread.start()

    # Run the plugin
    loop.run_until_complete(handshake(plugin, handles, comm))
    loop.run_until_complete(run(plugin, comm, read_thread, write_thread))
    plugin.logger.debug("Quit plugin while {} tasks were still running".format(len(asyncio.Task.all_tasks())))

    # Clean up everything
    write_thread.join()
    read_thread.join()
    sock.close()
    log.info("Finished running `{}`".format(name))
