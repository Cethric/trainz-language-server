package lang

import com.intellij.lang.Language

/**
 * Represents the TrainzConfig language used by Trainz (config.txt files).
 */
object TrainzConfigLanguage : Language("TrainzConfig", "text/x-trainzconfig") {
    private fun readResolve(): Any = TrainzConfigLanguage

    override fun getDisplayName(): String = "TrainzConfig"
}
