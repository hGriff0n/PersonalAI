
# standard imports
import asyncio
import typing

# third-part imports

# local imports
import communication
from rpc import typing as rpc_types


# TODO: There might be a better way to provide the communication handler
class Plugin(rpc_types.PluginBase):
    """
    Base class for all plugins

    Exports the `main` method which acts as a customization point for any code that needs to be run every so often
    Clients will overload this method to implement their "calling" code
    """

    def __init__(self, comm: communication.CommunicationHandler) -> None:
        self._comm = comm

    async def main(self) -> bool:
        """
        Entrypoint for any code that needs to be run semi-regularly
        """
        await asyncio.sleep(5)
        return True
