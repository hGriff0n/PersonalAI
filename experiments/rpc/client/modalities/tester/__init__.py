
# standard imports
import asyncio

# third-party imports

# local imports
from personal_ai import plugins
from personal_ai import rpc

class TestClient(plugins.Client):

    async def main(self) -> bool:
        await asyncio.sleep(5)

        rpc_message = rpc.Message(call="parley")
        print("Sending parley message")
        resp = await self._comm.wait_response(rpc_message)
        if resp is not None:
            print("Received {}".format(resp.resp))

        return False


class NullMessage(rpc.Serializable):

    def __init__(self):
        self.message: str = ""

    def serialize(self) -> rpc.SerializedMessage:
        return {
            'message': self.message
        }

    def deserialize(self, msg_dict: rpc.SerializedMessage) -> bool:
        self.message = msg_dict.get('message', '')
        return True


class FortuneCookie(plugins.Service):

    @rpc.endpoint
    async def grab_a_message(self, msg: NullMessage) -> NullMessage:
        msg.message = "This is a special message"
        return msg


class FrenchFortuneCookie(plugins.Service):

    @rpc.endpoint(name="parley")
    async def parlez(self, msg: NullMessage) -> NullMessage:
        fortune = await self._comm.wait_response(rpc.Message(call="grab_a_message"))
        if fortune is not None:
            message = NullMessage.from_dict(fortune.resp or {})
        if message is not None:
            msg.message = "C'est un message ({})".format(message.message)
        return msg
