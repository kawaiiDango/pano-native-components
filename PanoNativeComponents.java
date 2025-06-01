package com.arn.scrobble;

// all params should be non null
// this class is just useed for testing

public class PanoNativeComponents {
    private static native String ping(String input);

    private static native void startListeningMedia();

    private static native void startEventLoop(PanoNativeComponents callback);

    private static native void stopEventLoop();

    static native void setEnvironmentVariable(String key, String value);

    static native void setAllowedAppIds(String[] appIds);

    static native void stopListeningMedia();

    static native void skip(String appId);

    static native void mute(String appId);

    static native void unmute(String appId);

    static native void notify(String title, String body, String iconPath);

    static native void setTray(String tooltip, int[] argb, int icon_size, String[] menuItemIds, String[] menuItemTexts);

    static native String getMachineId();

    static native boolean isSingleInstance(String name);

    static native void applyDarkModeToWindow(long handle);

    static native void launchWebView(String url, String callbackPrefix, String dataDir);

    static native void getWebViewCookiesFor(String url);

    static native void quitWebView();

    static {
        System.loadLibrary("pano_native_components");
    }

    public static void main(String[] args) {
        System.out.println(ping("🪼hello " + getMachineId()));
        applyDarkModeToWindow(0);
        setEnvironmentVariable("GDK_BACKEND", "x11");
        boolean singleInstance = isSingleInstance("aaa");
        System.out.println("isSingleInstance: " + singleInstance);

        new Thread(new Runnable() {
            @Override
            public void run() {
                startEventLoop(new PanoNativeComponents());
            }
        }).start();

        new Thread(new Runnable() {
            @Override
            public void run() {
                while (true) {
                    try {
                        Thread.sleep(1000);
                    } catch (InterruptedException e) {
                        e.printStackTrace();
                    }
                    launchWebView("https://fonts.google.com", "callbackPrefix", "/tmp/webview");

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
                            // "org.mpris.MediaPlayer2.elisa",
                            "org.mpris.MediaPlayer2.plasma-browser-integration",
                            "com.apple.Music",
                            "org.mpris.MediaPlayer2.cider",
                            "org.mpris.MediaPlayer2.cider.instancen",

                });

                getWebViewCookiesFor("https://fonts.google.com");

                try {
                    Thread.sleep(5000);
                } catch (InterruptedException e) {
                    e.printStackTrace();
                }

                System.out.println("shutting down");
                // setAllowedAppIds(new String[] {});
                stopListeningMedia();
            }
        }).start();

    }

    public void onLogInfo(String msg) {
        System.out.println("info: " + msg);
    }

    public void onLogWarn(String msg) {
        System.err.println("warn: " + msg);
    }

    public void onActiveSessionsChanged(String json) {
        System.out.println("onActiveSessionsChanged: " + json);
    }

    public void onMetadataChanged(String json) {
        System.out.println("onMetadataChanged: " + json);
    }

    public void onPlaybackStateChanged(String json) {
        System.out.println("onPlaybackStateChanged: " + json);
    }

    public void onTrayMenuItemClicked(String id) {
        System.out.println("onTrayMenuItemClicked: " + id);
    }

    public void onWebViewCookies(String cookies) {
        System.out.println("onWebViewCookies: " + cookies);
    }

    public void onWebViewPageLoad(String url) {
        System.out.println("onWebViewPageLoad: " + url);
    }

}
