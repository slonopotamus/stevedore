import com.inet.gradle.setup.abstracts.Service
import de.undercouch.gradle.tasks.download.Download
import groovy.lang.Closure

val dockerVersion = "20.10.7"

group = "org.slonopotamus"
version = "${dockerVersion}.1"

plugins {
    id("com.github.ben-manes.versions") version "0.39.0"
    id("de.inetsoftware.setupbuilder") version "4.8.7"
    id("de.undercouch.download") version "4.1.2"
}

setupBuilder {
    licenseFile("LICENSE")
    vendor = "Marat Radchenko"
    service(closureOf<Service> {
        displayName = project.displayName
        executable = "dockerd.exe"
        startArguments = "--run-service --service-name $displayName --host npipe:////./pipe/docker_desktop_windows"
    } as Closure<Service>) // Workaround for https://github.com/i-net-software/SetupBuilder/pull/100
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

        // Workaround for https://github.com/i-net-software/SetupBuilder/issues/99
        inputs.file("stevedore.wxs")

        setWxsTemplate("stevedore.wxs")
    }

    wrapper {
        gradleVersion = "6.9"
        distributionType = Wrapper.DistributionType.ALL
    }

    assemble {
        dependsOn(msi)
    }
}
