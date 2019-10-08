
# standard imports
import logging
import typing

# third-part imports

# local imports

def create(file: str,
           name: typing.Optional[str] = None,
           log_dir: typing.Optional[str] = None,
           fmt: typing.Optional[str] = None,
           level: typing.Optional[str] = None) -> logging.Logger:
    """
    Create a logger with the specified arguments

    Provides default argument values to provide usable loggers
    """
    hdlr = logging.FileHandler("{}/{}".format(log_dir or '.', file))

    formatter = logging.Formatter(fmt or "[%(asctime)s][%(levelname)s][%(filename)s:%(lineno)d] %(message)s")
    hdlr.setFormatter(formatter)

    logger = logging.getLogger(name or __name__)
    logger.addHandler(hdlr)

    level = level or 'DEBUG'
    logger.setLevel(logging.getLevelName(level.upper()))

    return logger

def dummy_logger(*args, **kwargs):
       """
       Create a /dev/null logger that just swallows everything sent to it
       """
       log = logging.getLogger('NULL_dummy__NULL')
       log.addHandler(logging.NullHandler())
       return log

Logger = logging.Logger
