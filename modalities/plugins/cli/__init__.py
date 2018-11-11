
import asyncio
import shlex

from common.msg import Message
from common import plugins

class CliPlugin(plugins.Plugin):
    COMMAND = 0
    CHAT = 1

    def __init__(self, logger, config=None):
        super().__init__(logger, config=config)
        self._role = 'cli'

        self._msgs = []
        self._cli_lock = asyncio.Lock()

        # Handle registration
        self._register_handle('print', CliPlugin.handle_print)
        self._msg_handles = {

        }

        # CLI command system
        self._known_roles = []          # TODO: What is this for?
        self._handle_role_map = {       # TODO: Produce this list dynamically
            'dispatch': ['dispatch'],
            'play': ['audio'],
            'search': ['manager'],
            'stop': ['manager'],
            'quit': ['manager']
        }
        self._current_mode = self.CHAT

    # TODO: The message handling code needs to be put into a separate coroutine
    # Otherwise, we're just single threading the input
    async def run(self, comm):
        with await self._cli_lock:
            self._print_all_msgs()

        query = input("> ")
        if query == "":
            return True

        # Hard code in quit behavior
        # NOTE: This may not be necessary with "command mode"
        # I think this is because I couldn't get 'wit' to parse "quit" correctly
        elif query == "quit":
            self._log.info("Stopping all plugins from user input")

            msg = Message(plugin=self)
            msg.action = 'quit'
            msg.send_to(role='manager')
            comm.send(msg, self._log)

            return False

        # Handle any command manipulation
        elif query[0] == ':':
            self._log.debug("Detected cli operation command")
            await self._parse_cli_command(query)
            return True

        # In "chat" mode, we just forward everything to dispatch
        # This means, we can implement "chat" in terms of "command mode"
        with await self._cli_lock:
            if self._current_mode == self.CHAT:
                self._log.info("Translating chat query into dispatch command: {}".format(query))
                query = "dispatch \"{}\"".format(query)

        await self._parse_command(query, comm)

        return True

    async def _parse_cli_command(self, query):
        command = shlex.split(query)
        command, args = command[0][1:], command[1:]
        self._log.debug("Received cli command `{}`".format(command))

        if command == "alias":
            self._print_and_log("Aliasing cli command `{}` to expand to `{}`".format("unk", "unk"), 'info')

        elif command == "set-mode":
            mode = (args[0].lower() == "chat")
            self._print_and_log("Switching cli operation into {} mode".format(mode and 'CHAT' or 'COMMAND'), 'info', dont_print_log_level=True)

            with await self._cli_lock:
                self._current_mode = (mode and self.CHAT or self.COMMAND)

    async def _parse_command(self, query, comm):
        command = shlex.split(query)

        if len(command) == 0:
            return self._print_and_log("Received empty command: {}".format(query), 'error')

        # Parse out the handle (and role if provided)
        role = None
        handle, command = command[0], command[1:]
        # TODO: Need to handle aliases
        if ':' in handle:
            self._log.debug("Detected role namespacing in handle `{}`".format(handle))
            role, handle = tuple(handle.split(':'))

        # Extract the actual role that corresponds to the provided handle
        targets = self._handle_role_map.get(handle, [])
        if len(targets) == 0:
            return self._print_and_log("No targets found for handle `{}`".format(handle), 'error')

        # NOTE: The if check is for when `role is not None` (ie. the `or` is short-circuited)
        role = role or targets[0]
        if role not in targets:
            return self._print_and_log("Namespaced role `{}` not found as a valid target for handle `{}`".format(role, handle), 'error')
        self._log.info("Parsed command role={} and handle={}".format(role, handle))

        # Send the message
        # NOTE: For the moment, we're just relying on the user "knowning" the internal messages
        # TODO: We need to create a system for plugins to communicate a "parsing" system to this app
        # Or at least, provide a way to automatically translate these "command" messages through it
        msg = Message(plugin=self)
        msg.action = handle
        msg.args = command
        msg.send_to(role=role)
        resp = await comm.wait_for_response(msg, self._log)

        # NOTE: Do not acquire the cli lock before sending the message as that will deadlock some processing
        with await self._cli_lock:
            self._msgs.append(resp)

    def _print_and_log(self, msg, level, dont_print_log_level=None):
        log_method = getattr(self._log, level)
        log_method(msg)

        if dont_print_log_level:
            print(msg)
        else:
            print("{}: {}".format(level.capitalize(), msg))

    async def handle_print(self, msg, comm):
        self._log.info("Adding message to print queue")
        with await self._cli_lock:
            self._msgs.append(' '.join(msg.args))

    def _print_all_msgs(self):
        if len(self._msgs) != 0:
            for msg in self._msgs:
                print(msg.json_packet)

        self._msgs = []
