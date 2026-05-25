package lsp

import com.intellij.ide.util.PropertiesComponent
import com.intellij.openapi.project.Project
import com.redhat.devtools.lsp4ij.LanguageServerFactory
import com.redhat.devtools.lsp4ij.client.LanguageClientImpl
import com.redhat.devtools.lsp4ij.installation.CommandLineUpdater
import com.redhat.devtools.lsp4ij.installation.ServerInstaller
import com.redhat.devtools.lsp4ij.server.StreamConnectionProvider
import org.jetbrains.annotations.NotNull

/**
 * Factory that creates the [TrainzLanguageServerConnectionProvider] connection provider and wires
 * the declarative installer for the Trainz Language Server binary.
 *
 * [createServerInstaller] may be called by LSP4IJ *before* [createConnectionProvider],
 * so the installer receives a lazy proxy [CommandLineUpdater] that delegates to the
 * live [TrainzLanguageServerConnectionProvider.commandLineUpdater] at call time.  If the server has not
 * yet been created (unlikely in practice) the proxy falls back to writing directly to
 * [PropertiesComponent] so the installed path is still persisted for the next startup.
 */
class TrainzLanguageServerFactory : LanguageServerFactory {

    /** Shared server instance so the installer can update its command line in place. */
    private var server: TrainzLanguageServerConnectionProvider? = null

    @NotNull
    override fun createConnectionProvider(@NotNull project: Project): StreamConnectionProvider {
        return TrainzLanguageServerConnectionProvider(project).also { server = it }
    }

    @NotNull
    override fun createLanguageClient(@NotNull project: Project): LanguageClientImpl {
        return TrainzLanguageClient(project)
    }

    /**
     * Creates the server installer with a **lazy** proxy [CommandLineUpdater].
     *
     * The proxy resolves [server] at the moment [CommandLineUpdater.setCommandLine] is
     * invoked (i.e. when installation completes), not at construction time.  This ensures
     * the update reaches the live server even when [createServerInstaller] is called before
     * [createConnectionProvider].
     */
    @NotNull
    override fun createServerInstaller(): ServerInstaller {
        val lazyUpdater = object : CommandLineUpdater {
            override fun getCommandLine(): String =
                server?.commandLineUpdater?.commandLine ?: TrainzLanguageServerConnectionProvider.resolveServerPath()

            override fun setCommandLine(newPath: String) {
                val srv = server
                if (srv != null) {
                    // Update the live connection provider and persist via its updater.
                    srv.commandLineUpdater.commandLine = newPath
                } else {
                    // Server not yet created — persist directly so it is picked up on startup.
                    PropertiesComponent.getInstance().setValue(TrainzLanguageServerConnectionProvider.COMMAND_LINE_KEY, newPath)
                }
            }
        }
        return TrainzLanguageServerInstaller(lazyUpdater)
    }
}