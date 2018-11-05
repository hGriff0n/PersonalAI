
import asyncio
import json
import queue
import struct

from common.msg import Message

class MessageEvent(asyncio.Event):
    """
    Custom event for handling message responses
    """
    value = None


class CommChannel:
    """
    Custom communication channel to wrap client behaviors
    """
    def __init__(self, handles):
        self._event_queue = {}
        self._msg_queue = queue.Queue()
        self._handles = handles

    async def wait_for_response(self, msg, log):
        """
        Send a message to some other plugin and wait for a response message
        """
        self.send(msg, log)
        self._event_queue[msg.id] = MessageEvent()
        await self._event_queue[msg.id].wait()

        resp = self._event_queue[msg.id].value
        del self._event_queue[msg.id]
        return resp

    def send(self, msg, log):
        self._msg_queue.put(msg)

    def get_msg(self):
        return self._msg_queue.get()

    @property
    def handles(self):
        return self._handles

    @property
    def events(self):
        return self._event_queue


class JsonCodec:
    @staticmethod
    def send_message(msg, sock, log):
        """
        Automaticaly wrap message in correct json protocol
        """
        if isinstance(msg, Message):
            msg = msg.json_packet

        log.info("Sending message id={}: {}".format(msg.get('message_id'), msg))

        data = json.dumps(msg).encode('utf-8')
        frame = struct.pack(">I", len(data))
        sock.sendall(frame + data)

    @staticmethod
    def get_messages(sock, log):
        """
        Generator to automatically parse json protocol
        """
        try:
            while True:
                len_buf = sock.recv(4)
                msg_len = struct.unpack(">I", len_buf)[0]
                buf = sock.recv(msg_len)
                msg = json.loads(buf.decode('utf-8'))

                log.info("Received message id={}: {}".format(msg.get('message_id'), msg))
                yield Message.from_json(msg)

        except ConnectionResetError as e:
            log.error("Lost connection to server")

        except Exception as e:
            log.error("Exception while waiting for messages: {}".format(e))
