
These programs are responsible for managing device-level interaction capabilities, such as the cli and speaker/microphone. The responsibility of these components is to interleave input and output within their domain, sending user input to the interaction layer for decomposition and running received events with no input interference

## Configuration

Plugins can define their own configuration parsers by providing a 'cmd_args.yaml' file in the same directory as the
`__init__.py` file which defines the plugin. This file is then parsed by the 'clg' module to produce a parser which
is run on the leftover values from the loader's command line. The resultant configuration is passed into the plugin
through the 'config' argument of the constructor
