
# standard imports
import typing

# third-part imports

# local imports
from personal_ai import communication
from personal_ai import logger
from personal_ai.plugins import client, plugin, service


# TODO: Work on the loading procedure
# TODO: Handle cases where loading fails (What do I mean by this?)
def initialize_loaded_modality(args: typing.List[str],
                               comm: communication.CommunicationHandler,
                               log: logger.Logger,
                               log_dir: str) -> typing.List[plugin.Plugin]:
    services: typing.List[plugin.Plugin] = []
    log.debug("Initializing registered services: {}".format(service.get_registered_services()))
    services.extend(
        _create_plugin(plugin, args, comm, log_dir) for plugin in service.get_registered_services()
    )
    log.debug("Initializing registered clients: {}".format(service.get_registered_services()))
    services.extend(
        _create_plugin(client, args, comm, log_dir) for client in client.get_registered_clients()
    )
    log.info("Modality initialized")
    return services

def _create_plugin(plugin_cls: typing.Type[plugin.Plugin],
                   args: typing.List[str],
                   comm: communication.CommunicationHandler,
                   log_dir: str) -> plugin.Plugin:
    plugin_name = plugin_cls.__name__
    plugin_log = logger.create("{}.log".format(plugin_name), name=plugin_name, log_dir=log_dir)
    return plugin_cls(args, comm, plugin_log)
