package settings

import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.components.PersistentStateComponent
import com.intellij.openapi.components.State
import com.intellij.openapi.components.Storage
import com.intellij.util.xmlb.XmlSerializerUtil

/**
 * Application-level persistent settings for the Trainz Language Server.
 *
 * Each field maps directly to a CLI flag accepted by the `trainz-language-server` binary:
 *
 * | Field                | Flag                          | Env var                                         |
 * |----------------------|-------------------------------|-------------------------------------------------|
 * | [validationPath]     | `-p`/`--validation-path`      | `TRAINZ_LANGUAGE_SERVER_ACS_TEXT_VALIDATION_PATH` |
 * | [searchPaths]        | `-s`/`--search-paths`         | `TRAINZ_LANGUAGE_SERVER_SCRIPT_SEARCH_PATHS`    |
 * | [logFile]            | `--log-file`                  | `TRAINZ_LANGUAGE_SERVER_LOG_FILE`               |
 * | [logLevel]           | `--log-level`                 | `TRAINZ_LANGUAGE_SERVER_LOG_LEVEL`              |
 * | [assetCache]         | `--asset-cache`               | `TRAINZ_LANGUAGE_SERVER_ASSET_CACHE`            |
 * | [tdxCache]           | `--tdx-cache`                 | `TRAINZ_LANGUAGE_SERVER_TDX_CACHE`              |
 * | [extensionsOverrides]| `--extensions-overrides`      | `TRAINZ_LANGUAGE_SERVER_EXTENSIONS_OVERRIDES`   |
 */
@State(
    name = "TrainzLanguageServerSettings",
    storages = [Storage("trainz-language-server.xml")]
)
class TrainzLanguageServerSettings : PersistentStateComponent<TrainzLanguageServerSettings> {

    /** Path to the directory containing acs_text validators (`-p`/`--validation-path`). */
    var validationPath: String = ""

    /**
     * Semicolon-separated list of paths to search for Trainz scripts (`-s`/`--search-paths`).
     * The server accepts multiple values separated by `;`.
     */
    var searchPaths: String = ""

    /** Log file path (`--log-file`). Leave empty to log to stderr. */
    var logFile: String = ""

    /**
     * Log level (`--log-level`).
     * Accepted values: `error`, `warn`, `info`, `debug`, `trace` (or empty to use default).
     */
    var logLevel: String = ""

    /** Path to the asset cache SQLite file (`--asset-cache`). */
    var assetCache: String = ""

    /** Path to the TDX asset cache directory (`--tdx-cache`). */
    var tdxCache: String = ""

    /** Path to a folder for defining extension overrides (`--extensions-overrides`). */
    var extensionsOverrides: String = ""

    override fun getState(): TrainzLanguageServerSettings = this

    override fun loadState(state: TrainzLanguageServerSettings) {
        XmlSerializerUtil.copyBean(state, this)
    }

    companion object {
        /** Returns the application-level singleton instance. */
        fun getInstance(): TrainzLanguageServerSettings =
            ApplicationManager.getApplication()
                .getService(TrainzLanguageServerSettings::class.java)
    }
}
