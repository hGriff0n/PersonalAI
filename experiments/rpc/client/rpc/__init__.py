
import abc
import typing


UntypedMessage = typing.Dict[str, typing.Any]

class BaseMessage(object):
    @abc.abstractmethod
    def to_dict(self) -> UntypedMessage:
        """
        Convert this message to a dictionary (for generic storage and transmission)
        """

    @abc.abstractmethod
    def populate_from_dict(self, msg_dict: UntypedMessage) -> bool:
        """
        Populate this message from a given dictionary
        """

    M = typing.TypeVar('M', bound="BaseMessage")
    @classmethod
    def from_dict(kls: typing.Type[BaseMessage.M], msg_dict: UntypedMessage) -> typing.Optional[BaseMessage.M]:
        obj = kls()
        if not obj.populate_from_dict(msg_dict):
            return None
        return obj


# TODO: Make the members private and have accessors
class Message(BaseMessage):
    def __init__(self) -> None:
        self.msg_id: str = ""
        self._call: str = ""
        self.args: UntypedMessage = {}
        self.resp: typing.Optional[UntypedMessage] = None

    def to_dict(self) -> UntypedMessage:
        ret_dict = {
            'call': self._call,
            'args': self.args,
            'msg_id': self.msg_id,
        }
        if self.resp is not None:
            ret_dict['resp'] = self.resp
        return ret_dict

    def populate_from_dict(self, msg_dict: UntypedMessage) -> bool:
        if ('msg_vals' not in msg_dict) or ('call' not in msg_dict) or ('args' not in msg_dict):
            return False

        msg_vals = msg_dict.copy()
        self.msg_id = str(msg_vals.pop('msg_id'))
        self._call = str(msg_vals.pop('call'))
        self.args = dict(msg_vals.pop('args'))

        # `resp` is an optional value, so let's not throw on it
        self.resp = msg_vals.pop('resp', None)

        # If the provided msg def provides more keys than we expect
        # This is an invalid object, so let's return false
        return len(msg_vals) == 0

    @property
    def call(self) -> str:
        return self._call

    @call.setter
    def call(self, val: str) -> None:
        self._call = val
