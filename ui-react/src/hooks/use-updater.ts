import { relaunch } from '@tauri-apps/plugin-process'
import { check } from '@tauri-apps/plugin-updater'
import { useCallback, useEffect, useState } from 'react'
import { toast } from 'sonner'

export function useUpdater() {
  const [isChecking, setIsChecking] = useState(false)
  const [isDownloading, setIsDownloading] = useState(false)
  const [downloadProgress, setDownloadProgress] = useState(0)

  const downloadAndInstall = useCallback(async (update: Awaited<ReturnType<typeof check>>) => {
    if (!update) return

    setIsDownloading(true)
    setDownloadProgress(0)

    const toastId = toast.loading('Downloading update...', {
      description: '0%',
    })

    try {
      await update.downloadAndInstall((event) => {
        if (event.event === 'Started') {
          setDownloadProgress(0)
        } else if (event.event === 'Progress') {
          const progress = Math.round((event.data.downloaded / event.data.contentLength) * 100)
          setDownloadProgress(progress)
          toast.loading('Downloading update...', {
            id: toastId,
            description: `${progress}%`,
          })
        }
      })

      toast.success('Update installed successfully', {
        id: toastId,
        description: 'Restarting app...',
      })

      // Relaunch the app after a short delay
      setTimeout(async () => {
        await relaunch()
      }, 1000)
    } catch (error) {
      console.error('Failed to download and install update:', error)
      toast.error('Failed to install update', {
        id: toastId,
      })
    } finally {
      setIsDownloading(false)
    }
  }, [])

  const checkForUpdates = useCallback(
    async (showToast = true) => {
      if (isChecking || isDownloading) return

      setIsChecking(true)

      try {
        const update = await check()

        if (update?.available) {
          const version = update.version
          toast.info(`Update available: v${version}`, {
            description: 'Click to download and install',
            duration: Number.POSITIVE_INFINITY,
            action: {
              label: 'Update',
              onClick: () => downloadAndInstall(update),
            },
            cancel: {
              label: 'Later',
              onClick: () => {},
            },
          })
        } else if (showToast) {
          toast.success("You're up to date!")
        }
      } catch (error) {
        console.error('Failed to check for updates:', error)
        if (showToast) {
          toast.error('Failed to check for updates')
        }
      } finally {
        setIsChecking(false)
      }
    },
    [isChecking, isDownloading, downloadAndInstall]
  )

  useEffect(() => {
    // Check for updates on app startup (after a short delay)
    const checkTimeout = setTimeout(() => {
      checkForUpdates(false) // silent check on startup
    }, 3000)

    return () => clearTimeout(checkTimeout)
  }, [checkForUpdates])

  return {
    checkForUpdates,
    isChecking,
    isDownloading,
    downloadProgress,
  }
}
