import com.inet.gradle.setup.msi.Msi
import de.undercouch.gradle.tasks.download.Download

val dockerVersion = "20.10.7"

group = "org.slonopotamus"
version = "${dockerVersion}.1"

plugins {
    id("com.github.ben-manes.versions") version "0.39.0"
    id("de.inetsoftware.setupbuilder") version "4.8.7"
    id("de.undercouch.download") version "4.1.1"
}

tasks {
    val downloadDockerArchive by registering(Download::class) {
        val dockerArchive = "docker-${dockerVersion}.zip"
        src("https://download.docker.com/win/static/stable/x86_64/${dockerArchive}")
        dest(buildDir.resolve(dockerArchive))
        overwrite(false)
        tempAndMove(true)
    }

    val unzipDockerArchive by registering(Copy::class) {
        dependsOn(downloadDockerArchive)

        from(zipTree(downloadDockerArchive.get().dest))
        into(buildDir)
    }

    msi {
        dependsOn(unzipDockerArchive)
        from(buildDir.resolve("docker"))
        setLanguages("en-US")
        inputs.file("stevedore.wxs")
        setWxsTemplate("stevedore.wxs")
    }

    setupBuilder {
        vendor = "Marat Radchenko"
    }

    wrapper {
        gradleVersion = "6.9"
        distributionType = Wrapper.DistributionType.ALL
    }

    assemble {
        dependsOn(msi)
    }
}
