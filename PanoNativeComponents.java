package com.arn.scrobble;

// all params should be non null
// this class is just useed for testing

public class PanoNativeComponents {
    private static native String ping(String input);

    private static native void startListeningMedia();

    private static native void startEventLoop(PanoNativeComponents callback);

    static native void setAllowedAppIds(String[] appIds);

    static native void stopListeningMedia();

    static native void skip(String appId);

    static native void mute(String appId);

    static native void unmute(String appId);

    static native void notify(String title, String body, String iconPath);

    static native void setTrayIcon(int[] argb, int width, int height);

    static native void setTrayTooltip(String tooltip);

    static native void setTrayMenu(String[] menuItemIds, String[] menuItemTexts);

    static native String getMachineId();

    static native boolean addRemoveStartupWin(String exePath, boolean add);

    static native boolean isAddedToStartupWin(String exePath);

    static {
        System.loadLibrary("native_components");
    }

    public static void main(String[] args) {
        System.out.println(ping("hello " + getMachineId()));

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

                setTrayTooltip("tooltip");
                String menuItemIds[] = new String[] { "1", "2", "3", "Separator", "4" };
                String menuItemTexts[] = new String[] { "item1", "item2", "item3", "", "item4" };
                setTrayMenu(menuItemIds, menuItemTexts);

                // test tray icon

                int size = 8;
                int[] argb = new int[size * size];
                for (int i = 0; i < argb.length; i++) {
                    argb[i] = 0xffff0000;
                }
                setTrayIcon(argb, size, size);

                System.out.println("allowing apps");
                setAllowedAppIds(new String[] {
                            "Spotify.exe",
                            "MusicBee.exe",
                            "org.mpris.MediaPlayer2.Lollypop",
                            "org.mpris.MediaPlayer2.elisa",
                            "org.mpris.MediaPlayer2.plasma-browser-integration",
                });
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
        System.out.println("warn: " + msg);
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

}
