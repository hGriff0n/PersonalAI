#!/usr/bin/env python3

from setuptools import setup, find_packages
import sys

if sys.version_info >= (3, 7):
    print("PyAudio currently does not install with python versions >= 3.7. Setup is not supported")
    exit(1)

# TODO: Figure out how to get the rust exectuable built from this script
setup(name="PersonalAI",
    packages=[ 'rpc', 'plugins' ],
    url="https://github.com/hGriff0n/PersonalAI",
    description="personal ai platform",
    # long_description=open("README.md").read(),
    install_requires=[
        'pydantic',
        # 'wit',
        # 'PyAudio',
        # 'SpeechRecognition',
        # 'pypiwin32',
        # 'pydub',
        # 'clg',
        # 'anyconfig',
        # 'pyyaml'
    ],
#     dependency_links=[
#         'https://github.com/DeepHorizons/tts/tarball/master#egg=package-1.0'
#     ],
    # data_files=[('data', ['data/*'])],
    )

print("Be sure to install libav or ffmpeg for pydub to work on non-wav files")
print("Also know that the win32 package may not be installed correctly. Go onto the github page to download and run the post-installer for final setup")
