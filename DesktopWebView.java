package com.arn.scrobble;

// all params should be non null
// this class is just useed for testing

public class DesktopWebView {
    private static native void startEventLoop();

    static native void launchWebView(String url, String callbackPrefix, String dataDir);

    static native void getWebViewCookiesFor(String url);

    static native void quitWebView();

    static {
        System.loadLibrary("native_webview");
    }

    public static void main(String[] args) {

        launchWebView("https://fonts.google.com", "callbackPrefix", "/tmp/webview");
        
        new Thread(new Runnable() {
            @Override
            public void run() {
                startEventLoop();
            }
        }).start();

        try {
            Thread.sleep(5000);
        } catch (InterruptedException e) {
            e.printStackTrace();
        }

        getWebViewCookiesFor("https://fonts.google.com");

        try {
            Thread.sleep(5000);
        } catch (InterruptedException e) {
            e.printStackTrace();
        }
        quitWebView();
    }

    public static void onWebViewCookies(String url, String[] cookies) {
        System.out.println("onWebViewCookies: " + url);
        for (String cookie : cookies) {
            System.out.println("Cookie: " + cookie);
        }
    }

    public static void onWebViewUrlLoaded(String url) {
        System.out.println("onWebViewUrlLoaded: " + url);
    }

}
