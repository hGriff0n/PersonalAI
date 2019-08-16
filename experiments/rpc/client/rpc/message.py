
# standard imports
import abc
import typing
import uuid

# third-part imports

# local imports


# The "generic" communication medium for an rpc message is a simple dictionary
# The communication layer will be responsible for marshalling it to the required transprortation protocol
SerializedMessage = typing.Dict[str, typing.Any]


class Serializable(object):
    """
    The basic rpc "message" type

    This class exists solely to provide an easy way to "structure" rpc communication
    The most common use will be specifying `args` and `resp` types
    Endpoints will operate off of these types, the backend will handle conversion to/from
    """

    @abc.abstractmethod
    def serialize(self) -> SerializedMessage:
        """
        Convert this message to a dictionary for generic storage and transmission
        """

    @abc.abstractmethod
    def deserialize(self, msg_dict: SerializedMessage) -> bool:
        """
        Populate this message from the serialized representation

        Should return False iff the message could not be populated from the serialized representation
        Generally this means a required arg is missing, or unsupported args exist
        """

    M = typing.TypeVar('M', bound="Serializable")
    @classmethod
    def from_dict(kls: typing.Type['Serializable.M'], msg_dict: SerializedMessage) -> typing.Optional['Serializable.M']:
        obj = kls()
        if not obj.deserialize(msg_dict):
            return None
        return obj


class Message(Serializable):
    """
    The rpc communication type

    This is what gets received from, and sent to, the server
    """

    def __init__(self,
                 call: typing.Optional[str] = None,
                 args: typing.Optional[SerializedMessage] = None,
                 resp: typing.Optional[SerializedMessage] = None) -> None:
        self._msg_id = uuid.uuid4()
        self._call = call or ""
        self._args = args or {}
        self._resp: typing.Optional[SerializedMessage] = resp

    def serialize(self) -> SerializedMessage:
        ret_dict = {
            'call': self._call,
            'args': self._args,
            'msg_id': self._msg_id,
        }
        if self._resp is not None:
            ret_dict['resp'] = self._resp
        return ret_dict

    def deserialize(self, msg_dict: SerializedMessage) -> bool:
        if any(key not in msg_dict for key in ['msg_id', 'call', 'args']):
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
    def args(self) -> SerializedMessage:
        return self._args

    @property
    def resp(self) -> typing.Optional[SerializedMessage]:
        return self._resp

    @resp.setter
    def resp(self, val: SerializedMessage) -> None:
        self._resp = val
