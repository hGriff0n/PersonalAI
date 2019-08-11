
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
    def from_dict(kls: typing.Type['BaseMessage.M'], msg_dict: UntypedMessage) -> typing.Optional['BaseMessage.M']:
        obj = kls()
        if not obj.populate_from_dict(msg_dict):
            return None
        return obj


class Message(BaseMessage):

    def __init__(self,
                 msg_id: typing.Optional[str] = None,
                 call: typing.Optional[str] = None,
                 args: typing.Optional[UntypedMessage] = None,
                 resp: typing.Optional[UntypedMessage] = None) -> None:
        self._msg_id = msg_id or ""
        self._call = call or ""
        self._args = args or {}
        self._resp: typing.Optional[UntypedMessage] = resp

    def to_dict(self) -> UntypedMessage:
        ret_dict = {
            'call': self._call,
            'args': self._args,
            'msg_id': self._msg_id,
        }
        if self._resp is not None:
            ret_dict['resp'] = self._resp
        return ret_dict

    def populate_from_dict(self, msg_dict: UntypedMessage) -> bool:
        if ('msg_id' not in msg_dict) or ('call' not in msg_dict) or ('args' not in msg_dict):
            return False

        msg_vals = msg_dict.copy()
        self._msg_id = str(msg_vals.pop('msg_id'))
        self._call = str(msg_vals.pop('call'))
        self._args = dict(msg_vals.pop('args'))

        # `resp` is an optional value, so let's not throw on it
        self._resp = msg_vals.pop('resp', None)

        # If the provided msg def provides more keys than we expect
        # This is an invalid object, so let's return false
        return len(msg_vals) == 0

    @property
    def call(self) -> str:
        return self._call

    @call.setter
    def call(self, val: str) -> None:
        self._call = val

    @property
    def msg_id(self) -> str:
        return self._msg_id

    @property
    def args(self) -> UntypedMessage:
        return self._args

    @property
    def resp(self) -> typing.Optional[UntypedMessage]:
        return self._resp

    @resp.setter
    def resp(self, val: UntypedMessage) -> None:
        self._resp = val


##
## Service decorators
##

# This class enables typing restrictions on the endpoint decorators
# We can't use the actual Plugin base class because of dependencies on communication (which depends on us)
class PluginBase(object):
    pass
