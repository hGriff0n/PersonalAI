
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
        self._known_roles = []
        self._handle_role_map = {}
        self._current_mode = self.CHAT

    # TODO: The message handling code needs to be put into a separate coroutine
    # Otherwise, we're just single threading the input
    async def run(self, comm):
        with await self._cli_lock:
            self._print_all_msgs()

        query = input("> ")
        if query == "":
            return True

        elif query == "quit":
            self._log.info("Stopping all plugins from user input")

            msg = Message(plugin=self)
            msg.action = 'quit'
            msg.send_to(role='manager')
            comm.send(msg, self._log)

            return False

        with await self._cli_lock:
            if self._current_mode == self.CHAT:
                query = "dispatch \"{}\"".format(query)

        await self._parse_command(query, comm)

        return True

    async def _parse_cli_command(self, command, args):
        if command == "alias":
            # TODO: Implement
            pass

        elif command == "set-mode":
            mode = (args.lower() == "chat")

            with await self._cli_lock:
                self._current_mode = (mode and self.CHAT or self.COMMAND)

        return

    async def _parse_command(self, query, comm):
        command = shlex.split(query)

        if len(command) == 0:
            # TODO: Error
            return

        # Handle any command manipulation
        if command[0][0] == ':':
            return self._parse_cli_command(command[0][1:], command[1:])

        # Parse out the handle (and role if provided)
        role = None
        handle, command = command[0], command[1:]
        # TODO: Need to handle aliases
        if ':' in handle:
            role, handle = tuple(handle.split(':'))

        # Extract the actual role that corresponds to the provided handle
        targets = self._handle_role_map.get(handle, [])
        if len(targets) == 0:
            # TODO: Error
            return

        # NOTE: The if check is for when `role is not None` (ie. the `or` is short-circuited)
        role = role or targets[0]
        if role not in targets:
            # TODO: Error
            return

        # Send the message
        # NOTE: For the moment, we're just relying on the user "knowning" the internal messages
        # TODO: We need to create a system for plugins to communicate a "parsing" system to this app
        # Or at least, provide a way to automatically translate these "command" messages through it
        msg = Message(plugin=self)
        msg.action = handle
        msg.args = command
        msg.send_to(role=role)
        resp = await comm.wait_for_response(msg, self._log)

        with await self._cli_lock:
            self._msgs.append(resp)

    async def handle_print(self, msg, comm):
        self._log.info("Adding message to print queue")
        with await self._cli_lock:
            self._msgs.append(' '.join(msg.args))

    def _print_all_msgs(self):
        if len(self._msgs) != 0:
            for msg in self._msgs:
                print(msg.json_packet)

        self._msgs = []
