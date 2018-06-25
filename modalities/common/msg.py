
from collections.abc import MutableMapping

# TODO: Adapt this to be stricter about allowable interface, but looser about allowed interface
class Message(MutableMapping):
    def __init__(self, sender):
        if isinstance(sender, dict):
            self.msg = sender

        else:
            self.msg = {
                'from': sender,
                'routing': 'sender',
                'stop': False
            }

    def send_to(self, dest):
        self['routing'] = dest

    def message(self, text):
        self['text'] = text

    def action(self, key):
        self['action'] = key

    def dispatch(self, query):
        self.action('dispatch')
        self.send_to('dispatch')
        self['dispatch'] = query

    def finalize(self):
        return self.msg

    @staticmethod
    def quit():
        return "quit"

    @staticmethod
    def stop():
        return "stop"

    @staticmethod
    def is_quit(msg):
        return msg == Message.quit() or msg == Message.stop()


    # Methods to make the message mimic it's underlying dict typing
    def __str__(self):
        return str(self.msg)

    def __delitem__(self, key):
        del self.msg[key]

    def __getitem__(self, key):
        return self.msg[key]

    def __iter__(self):
        return iter(self.msg)

    def __len__(self):
        return len(self.msg)

    def __setitem__(self, key, value):
        self.msg[key] = value
