from setuptools import setup, find_packages

# TODO: I have no idea whether this is accurate or not
setup(
    name='PersonalAI',
    packages=find_packages(),
    url='https://github.com/hGriff0n/PersonalAI',
    description='TODO',
    long_description=open('README.md').read(),
    install_requires=[
        "wit>=5.1",
        "SpeechRecognition>=3.5",
    ],
    dependency_links=[
        'https://github.com/DeepHorizons/tts/tarball/master#egg=package-1.0'
    ],
    include_package_data=True,
)
