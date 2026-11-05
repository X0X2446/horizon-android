package com.horizon.android

import android.app.NativeActivity
import android.content.pm.ActivityInfo
import android.hardware.Sensor
import android.hardware.SensorEvent
import android.hardware.SensorEventListener
import android.hardware.SensorManager
import android.os.Bundle
import android.view.SurfaceHolder
import android.view.SurfaceView

/**
 * The game's activity: a NativeActivity subclass.
 *
 * NativeActivity calls `android_main` in the Rust code when the activity is created.
 * The Rust code reads `horizon-launch.json` from the files directory and runs the game.
 *
 * This activity also:
 * - Feeds the gyroscope samples to the native side (when gyroscope_control is on).
 * - Tells the native side the screen size on surface changes.
 * - Handles pause/resume (the game releases its touches while paused).
 * - Applies sustained performance mode (the compatibility setting).
 */
class GameActivity : NativeActivity(), SurfaceHolder.Callback, SensorEventListener {

    private var sensorManager: SensorManager? = null
    private var rotationVector: Sensor? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Fullscreen immersive: the game is the whole screen.
        requestedOrientation = ActivityInfo.SCREEN_ORIENTATION_LANDSCAPE

        // The surface view: NativeActivity creates the surface, but we need a
        // SurfaceHolder to receive surface callbacks.
        val surfaceView = SurfaceView(this)
        surfaceView.holder.addCallback(this)
        setContentView(surfaceView)

        // The gyroscope: a rotation vector sensor feeds the native side.
        sensorManager = getSystemService(SENSOR_SERVICE) as SensorManager
        rotationVector = sensorManager?.getDefaultSensor(Sensor.TYPE_ROTATION_VECTOR)
        if (rotationVector != null) {
            sensorManager?.registerListener(this, rotationVector, SensorManager.SENSOR_DELAY_GAME)
        }
    }

    override fun surfaceCreated(holder: SurfaceHolder) {
        // The surface is ready: the native side will create the window.
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
        nativeSetScreenSize(width, height)
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        // The surface is gone: the game should release its touches.
    }

    override fun onPause() {
        super.onPause()
        nativeOnPaused()
        sensorManager?.unregisterListener(this)
    }

    override fun onResume() {
        super.onResume()
        nativeOnResumed()
        if (rotationVector != null) {
            sensorManager?.registerListener(this, rotationVector, SensorManager.SENSOR_DELAY_GAME)
        }
    }

    override fun onSensorChanged(event: SensorEvent) {
        if (event.sensor.type == Sensor.TYPE_ROTATION_VECTOR) {
            nativeOnGyroscope(
                event.values[0],
                event.values[1],
                event.values[2],
                event.timestamp
            )
        }
    }

    override fun onAccuracyChanged(sensor: Sensor?, accuracy: Int) {}

    // --- JNI (libhorizon.so) ---

    private external fun nativeSetScreenSize(width: Int, height: Int)
    private external fun nativeOnPaused()
    private external fun nativeOnResumed()
    private external fun nativeOnGyroscope(x: Float, y: Float, z: Float, t: Long)
    private external fun nativeVersion(): String
    private external fun nativeGameState(): String
    private external fun nativeQuit()

    companion object {
        init {
            System.loadLibrary("horizon")
        }
    }
}
