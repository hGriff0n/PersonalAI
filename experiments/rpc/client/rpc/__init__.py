
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
    def populate_from_dict(self, msg_dict: UntypedMessage) -> None:
        """
        Populate this message from a given dictionary
        """

    M = typing.TypeVar('M', bound="BaseMessage")
    @classmethod
    def from_dict(kls: typing.Type[BaseMessage.M], msg_dict: UntypedMessage) -> BaseMessage.M:
        obj = kls()
        obj.populate_from_dict(msg_dict)
        return obj


# TODO: Should I write my python with type annotations?
# TODO: Make the members private and have accessors
class Message(BaseMessage):
    def __init__(self) -> None:
        self._call = ""
        self.args: UntypedMessage = {}
        self.resp = None
        self.msg_id = ""

    def to_dict(self) -> UntypedMessage:
        ret_dict = {
            'call': self._call,
            'args': self.args,
            'msg_id': self.msg_id,
        }
        if self.resp is not None:
            ret_dict['resp'] = self.resp
        return ret_dict

    # TODO: How do you get from a dictionary in a typed manner?
    def populate_from_dict(self, msg_dict: UntypedMessage) -> None:
        self.msg_id = msg_dict.get('msg_id', "")
        self._call = str(msg_dict.get('call', ""))
        self.args = msg_dict.get('args', {})
        self.resp = msg_dict.get('resp')

    @property
    def call(self) -> str:
        return self._call

    @call.setter
    def call(self, val: str) -> None:
        self._call = val
