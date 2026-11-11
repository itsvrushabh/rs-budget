# Proguard rules for Actual Budget Mobile (NativeActivity)
-keep class android.app.NativeActivity { *; }
-keepclassmembers class * extends android.app.NativeActivity {
    *;
}
-dontwarn android.app.NativeActivity
