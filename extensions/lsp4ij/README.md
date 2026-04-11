# Development Support

The current development server is tested using [the LSP4IJ](https://plugins.jetbrains.com/plugin/23257-lsp4ij) plugin.
This plugin provides a seamless integration between the development server and the IDE, allowing for efficient language
server protocol (LSP) support.

The [lsp4ij-configuration.zip](./lsp4ij-configuration.zip) contains the configuration files for the LSP4IJ plugin. To
use this configuration, follow these steps:

1. Install the LSP4IJ plugin from the JetBrains Marketplace.
2. Import the configuration files into your IDE (Select the folder) by going to
   `File > Settings > Languages & Frameworks > Language Servers > Add Language Server`.
3. Open a `.gs` or `.txt` file and the server will start automatically.