
# standard imports
import asyncio

# third-part imports

# local imports
from personal_ai import communication
from personal_ai import logger
from personal_ai.rpc.typing import PluginBase


# TODO: There might be a better way to provide the communication handler
class Plugin(PluginBase):
    """
    Base class for all plugins

    Exports the `main` method which acts as a customization point for any code that needs to be run every so often
    Clients will overload this method to implement their "calling" code
    """

    def __init__(self, comm: communication.CommunicationHandler, log: logger.Logger) -> None:
        self._comm = comm
        self._log = log

    async def main(self) -> bool:
        """
        Entrypoint for any code that needs to be run semi-regularly
        """
        await asyncio.sleep(5)
        return True
