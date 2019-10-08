
# standard imports
import asyncio
import typing

# third-part imports

# local imports
from personal_ai import communication
from personal_ai import logger
from personal_ai.rpc.typing import PluginBase


class Plugin(PluginBase):
    """
    Base class for all plugins

    Exports the `main` method which acts as a customization point for any code that needs to be run every so often
    Clients will overload this method to implement their "calling" code

    TODO: There might be a better way to provide the communication handler
    """

    def __init__(self, args: typing.List[str], comm: communication.CommunicationHandler, log: logger.Logger) -> None:
        self._comm = comm
        self._log = log

        _unused = args

    async def main(self) -> bool:
        """
        Entrypoint for any code that needs to be run semi-regularly
        """
        await asyncio.sleep(5)
        return True
