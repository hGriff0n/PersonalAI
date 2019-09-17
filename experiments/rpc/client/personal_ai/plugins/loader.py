
# standard imports
import typing

# third-part imports

# local imports
from personal_ai import communication
from personal_ai.plugins import client, plugin, service


# TODO: Work on the loading procedure
# TODO: Incorporate configuration into this
# TODO: Handle cases where loading fails (What do I mean by this?)
def initialize_registered_plugins(comm: communication.CommunicationHandler) -> typing.List[plugin.Plugin]:
    services: typing.List[plugin.Plugin] = []
    services.extend(
        plugin(comm) for plugin in service.get_registered_services()
    )
    services.extend(
        client(comm) for client in client.get_registered_clients()
    )
    return services
