
# standard imports
import typing

# third-part imports

# local imports
from plugins import plugin


_REGISTERED_CLIENTS: typing.List[typing.Type[plugin.Plugin]] = []
def get_registered_clients() -> typing.List[typing.Type[plugin.Plugin]]:
    return _REGISTERED_CLIENTS


class Client(plugin.Plugin):
    """
    Definition class which marks all plugins that inherit from it as a clients
    Clients only interact with the wider network to send rpc calls (as opposed to servers)

    NOTE: This class should only be inherited on leaf nodes
    """

    def __init_subclass__(cls, **kwargs):
        super().__init_subclass__(**kwargs)

        global _REGISTERED_CLIENTS
        _REGISTERED_CLIENTS.append(cls)
