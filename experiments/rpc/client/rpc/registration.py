# standard imports
import inspect
import typing

# third-part imports

# local imports
from rpc import message
from rpc import typing as rpc_type


# Endpoint registration structures
RpcEndpoint = typing.Dict[str, str]
__ACTIVE_ENDPOINT_REGISTRATIONS: typing.Dict[str, RpcEndpoint]
__REGISTERED_ENDPOINTS: typing.Dict[typing.Type[rpc_type.PluginBase], typing.Dict[str, RpcEndpoint]] = {}

# TODO: Not sure on the interface for creating the dispatcher
def endpoints_for_class(kls: typing.Type[rpc_type.PluginBase]) -> typing.Dict[str, RpcEndpoint]:
    """
    Get the list of registered endpoitns for the service
    """
    return __REGISTERED_ENDPOINTS.get(kls, {})


##
## Service Registration
##

def service(kls: typing.Type[rpc_type.PluginBase]):
    """
    Class decorator that maps the active list of registered endpoints to the specific plugin service

    NOTE: Decorators are only run once!!! This will not work with inheritance
    This is acceptable as `service` should be used to indicate a "specific" service, not a family of services
    """

    global __ACTIVE_ENDPOINT_REGISTRATIONS
    global __REGISTERED_ENDPOINTS

    __REGISTERED_ENDPOINTS[kls] = __ACTIVE_ENDPOINT_REGISTRATIONS
    __ACTIVE_ENDPOINT_REGISTRATIONS = {}
    return kls


def get_registered_services() -> typing.KeysView[typing.Type[rpc_type.PluginBase]]:
    return __REGISTERED_ENDPOINTS.keys()


##
## Endpoint Registration
##

def _register_endpoint(func: rpc_type.AllEndpointTypes, rpc_name: typing.Optional[str]) -> None:
    """
    Adds the function to active endpoint registration tracker, with an ability to rename the exported endpoint
    """

    global __ACTIVE_ENDPOINT_REGISTRATIONS

    name = rpc_name if rpc_name else func.__name__
    if name in __ACTIVE_ENDPOINT_REGISTRATIONS:
        raise Exception("Name clash on endpoint `{}`: Two endpoints found with the same rpc name".format(name))

    __ACTIVE_ENDPOINT_REGISTRATIONS[name] = {
        'func': func.__name__
    }


def _extract_endpoint_types(func):
    """
    Extract the argument and return types of the endpoint function
    This is used to add in automatic casting from the message arguments

    NOTE: THis is explicitly not typed as it is not possible to safely do this in the current iteration of `typing`
    We instead rely on inspection to extract the type annotations and return them
    """

    fn_sig = inspect.signature(func)
    ret_ty = fn_sig.return_annotation
    if ret_ty == inspect.Signature.empty:
        ret_ty = None

    try:
        arg_ty = list(fn_sig.parameters.items())[1][1].annotation
    except:
        arg_ty = None
    return arg_ty, ret_ty


def endpoint(_func: typing.Optional[rpc_type.AllEndpointTypes] = None, name: typing.Optional[str] = None):
    """
    Function decorator to take a service endpoint definition, wrap it in the required disptach type, and register it

    From the plugin perspective, endpoints should take in a specific argument type and return a specific return type
    From the dispatcher perspective, endpoints should take in a `rpc.Message` and return a `rpc.Message`
    This decorator automatically inserts the steps required to translate between the two perspective

    NOTE: Decorators are only run once!!! This will not work with inheritance
    This is acceptable as an `endpoint` should represent a specific callable behavior, sharing makes no sense
    """

    def _decorator_no_type_checks(func):
        """
        Create the actual wrapper for the endpoint
        This wrapper automatically handles parsing of the message arguments to/from the rpc.Message network type
        This also handles any exceptions thrown by the function and reports them as errors

        NOTE: This is explicitly implemented with type checks off because of the split behavior with no-arg endpoints
        """
        arg_ty, _ret_ty = _extract_endpoint_types(func)

        # Add error reporting and return parsing to the no-arg endpoint function
        async def _call_no_arg(plugin) -> typing.Optional[message.SerializedMessage]:
            try:
                resp = await func(plugin)
                if resp is not None:
                    return resp.serialize()
                return None

            except Exception as e:
                return {'error': str(e)}

        # Add error reporting, return parsing, and argument parsing to the arg endpoint function
        async def _call_with_arg(plugin, args: message.SerializedMessage) -> typing.Optional[message.SerializedMessage]:
            try:
                typed_args = arg_ty.from_dict(args)
                if typed_args is None:
                    return {'error': "failed to parse arguments as a {}".format(arg_ty)}

                resp = await func(plugin, typed_args)
                if resp is not None:
                    return resp.serialize()
                return None

            except Exception as e:
                return {'error': str(e)}

        # This is the actual function that get's sent to the rpc registration
        async def _wrapper(self, msg: message.Message) -> message.Message:
            if arg_ty is None:
                msg.resp = await _call_no_arg(self)
            else:
                msg.resp = await _call_with_arg(self, msg.args)
            return msg

        # Set the name and docstring of the wrapped function
        # NOTE: functools.wraps also modifies the signature which we actually don't want
        _wrapper.__name__ = func.__name__
        _wrapper.__doc__ = func.__doc__
        return _wrapper

    # Decorator implementation
    # This handles registration and type checking of arguments
    def _decorator(func: rpc_type.AllEndpointTypes) -> rpc_type.DispatcherEndpointType:
        _register_endpoint(func, name)
        return _decorator_no_type_checks(func)

    if _func is None:
        return _decorator
    return _decorator(_func)
