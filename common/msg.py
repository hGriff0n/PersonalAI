
import collections
import types
import uuid

class Message:
    """
    Wrap a json dictionary to ensure adherence to the network's message protocol
    """
    def __init__(self, plugin=None, role=None):
        self._msg = {
            'message_id': str(uuid.uuid4()),
            'route': [],
            'sender': {},
            'dest': {},
        }
        if plugin is not None:
            self._msg['sender']['uuid'] = plugin.uuid
        if role is not None:
            self._msg['sender']['role'] = role

    @property
    def json_packet(self):
        return self._msg

    @staticmethod
    def from_json(json_msg):
        msg = Message()
        msg._msg = json_msg.copy()
        return msg

    @property
    def routing(self):
        routing_info = {}
        for key in ['sender', 'route', 'dest', 'forward', 'message_id', 'parent_id', 'ack_uuid']:
            if key in self._msg:
                routing_info[key] = self._msg[key].copy()
        return routing_info

    @property
    def id(self):
        return self._msg['message_id']

    @property
    def parent_id(self):
        return self._msg.get('parent_id')

    @parent_id.setter
    def parent_id(self, pid):
        self._msg['parent_id'] = pid

    @property
    def action(self):
        return self._msg.get('action')

    @action.setter
    def action(self, action):
        self._msg['action'] = action

    @property
    def args(self):
        if 'args' not in self._msg:
            self._msg['args'] = []

        return self._msg['args']

    @args.setter
    def args(self, args):
        if isinstance(args, tuple) or isinstance(args, types.GeneratorType) or isinstance(args, collections.KeysView):
            args = list(args)

        elif not isinstance(args, list):
            args = [ args ]

        self._msg['args'] = args

    @property
    def response(self):
        return self._msg.get('resp')

    @response.setter
    def response(self, resp):
        self._msg['resp'] = resp

    @property
    def broadcast(self):
        return self._msg['dest'].get('broadcast', False)

    @broadcast.setter
    def broadcast(self, value):
        self._msg['dest']['broadcast'] = bool(value)


    def return_to_sender(self):
        """
        Immediately sets the destination routing field to be the same as the sender field
        This has the side-effect of forcing routing to return the message to the sender
        """
        self._msg['dest'] = self._msg['sender'].copy()

    def send_to(self, role=None, addr=None, uuid=None, intra_device=None):
        """
        Adds routing request information to the destination field of the message
        NOTE: Depending on the fields used, this will not guarantee sending to a specific app
        """
        if role is not None:
            self._msg['dest']['role'] = role
        if addr is not None:
            self._msg['dest']['addr'] = addr
        if uuid is not None:
            self._msg['dest']['uuid'] = uuid
        if intra_device is not None:
            self._msg['dest']['intra_device'] = intra_device

    @staticmethod
    def is_quit(msg):
        if not isinstance(msg, Message):
            return True
        return msg.action == Message.QUIT or msg.action == Message.STOP

    STOP = "stop"
    QUIT = "quit"
