
import uuid

class Message:
    def __init__(self, msg=None):
        if msg is None:
            self._msg = {
                'message_id': uuid.uuid4(),
                'route': []
            }
        else:
            self._msg = msg

    def handler(self, role):
        self._msg['sender']['role'] = role

    def set_sender(self, plugin, role):
        self._msg['sender'] = {
            'uuid' = plugin.uuid,
            'role' = role
        }

    def return_to_sender(self):
        self._msg['dest'] = self._msg['sender']

    def set_destination(self, role=None, addr=None, uuid=None, intra_device=None):
        self._msg['dest'] = {}
        if role is not None:
            self._msg['dest']['role'] = role
        if addr is not None:
            self._msg['dest']['addr'] = addr
        if uuid is not None:
            self._msg['dest']['uuid'] = uuid
        if intra_device is not None:
            self._msg['dest']['intra_device'] = intra_device

    def set_parent_id(self, parent):
        self._msg['parent_id'] = parent

    def set_ack_uuid(self, msg_uuid):
        self._msg['ack_uuid'] = msg_uuid

    @property
    def id(self):
        return self._msg['message_id']

    @property
    def action(self):
        return self._msg['action']

    def set_action(self, action):
        self._msg['action'] = action

    @property
    def args(self):
        return self._msg['args']

    def set_args(self, *args):
        self._msg['args'] = args

    def set_stop(self):
        self._msg['stop'] = True

    @property
    def response(self):
        return self._msg['resp']

    def set_response(self, resp):
        self._msg['resp'] = resp

    @staticmethod
    def is_quit(msg):
        if not isinstance(msg, Message):
            return True
        return msg.action == Message.QUIT or msg.action == Message.STOP

    STOP = "stop"
    QUIT = "quit"
