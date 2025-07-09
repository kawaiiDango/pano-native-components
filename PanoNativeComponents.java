package com.arn.scrobble;

// all params should be non null
// this class is just useed for testing

public class PanoNativeComponents {
    private static native String ping(String input);

    private static native void startListeningMedia();

    static native void setEnvironmentVariable(String key, String value);

    static native void setAllowedAppIds(String[] appIds);

    static native void albumArtEnabled(boolean enabled);

    static native void stopListeningMedia();

    static native void skip(String appId);

    static native void mute(String appId);

    static native void unmute(String appId);

    static native void notify(String title, String body, String iconPath);

    static native void setTray(String tooltip, int[] argb, int icon_size, String[] menuItemIds, String[] menuItemTexts);

    static native String getMachineId();

    static native void applyDarkModeToWindow(long handle);

    static native boolean sendIpcCommand(String command, String arg);

    static native boolean isFileLocked(String path);

    static native String getSystemLocale();

    static {
        System.loadLibrary("pano_native_components");
    }

    public static void main(String[] args) {
        System.out.println(ping("🪼hello " + getMachineId()));
        System.out.println("locale: " + getSystemLocale());
        applyDarkModeToWindow(0);
        setEnvironmentVariable("GDK_BACKEND", "x11");

        sendIpcCommand("testCommand", "testArg");

        new Thread(new Runnable() {
            @Override
            public void run() {
                while (true) {
                    try {
                        Thread.sleep(1000);
                    } catch (InterruptedException e) {
                        e.printStackTrace();
                    }
                    
                    System.out.println("startListeningMedia");
                    startListeningMedia();
                    System.out.println("startListeningMedia finished");
                }
            }
        }).start();

        new Thread(new Runnable() {
            @Override
            public void run() {
                try {
                    Thread.sleep(5000);
                } catch (InterruptedException e) {
                    e.printStackTrace();
                }

                albumArtEnabled(true);

                // test tray icon
                String menuItemIds[] = new String[] { "1", "2", "3", "Separator", "4" };
                String menuItemTexts[] = new String[] { "📝 item1", "item2", "item3", "", "item4" };

                int size = 8;
                int[] argb = new int[size * size];
                for (int i = 0; i < argb.length; i++) {
                    argb[i] = 0xffbebebe;
                }
                setTray("tooltip", argb, size, menuItemIds, menuItemTexts);

                System.out.println("allowing apps");
                setAllowedAppIds(new String[] {
                            "Spotify.exe",
                            "MusicBee.exe",
                            "foobar2000.exe",
                            "AppleInc.AppleMusicWin_nzyj5cx40ttqa!App",
                            "org.mpris.MediaPlayer2.Lollypop",
                            "org.mpris.MediaPlayer2.elisa",
                            "org.mpris.MediaPlayer2.plasma-browser-integration",
                            "com.apple.Music",
                            "org.mpris.MediaPlayer2.cider",
                            "org.mpris.MediaPlayer2.cider.instancen",

                });

                try {
                    Thread.sleep(5000);
                } catch (InterruptedException e) {
                    e.printStackTrace();
                }

                System.out.println("shutting down");
                // setAllowedAppIds(new String[] {});
            }
        }).start();

    }

    public static void onActiveSessionsChanged(String[] appIds, String[] appNames) {
        System.out.println("onActiveSessionsChanged: ");
        for (int i = 0; i < appIds.length; i++) {
            System.out.println("App ID: " + appIds[i] + ", App Name: " + appNames[i]);
        }
    }

    public static void onMetadataChanged(String appId, String title, String artist, String album, String albumArtist, int trackNumber, long duration, String artUrl, byte[] artBytes) {
        System.out.println("onMetadataChanged: " + appId + ", " + title + ", " + artist + ", " + album + ", " + albumArtist + ", " + trackNumber + ", " + duration + ", " + artUrl + ", " + (artBytes != null ? artBytes.length : 0) + " bytes");
    }

    public static void onPlaybackStateChanged(String appId, String state, long position, boolean canSkip) {
        System.out.println("onPlaybackStateChanged: " + appId + ", " + state + ", " + position + ", " + canSkip);
    }

    public static void onTrayMenuItemClicked(String id) {
        System.out.println("onTrayMenuItemClicked: " + id);
    }

    public static void onReceiveIpcCommand(String command, String arg) {
        System.out.println("onReceiveIpcCommand: " + command + " " + arg);
    }

}
