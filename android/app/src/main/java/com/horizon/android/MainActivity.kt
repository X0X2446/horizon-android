package com.horizon.android

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import android.view.View
import android.widget.AdapterView
import android.widget.ArrayAdapter
import android.widget.CheckBox
import android.widget.EditText
import android.widget.SeekBar
import android.widget.Spinner
import android.widget.Toast
import java.io.File

/**
 * The launcher: the settings screen, the world list, the play button.
 *
 * The launcher writes `horizon-launch.json` into the app's files directory and starts
 * [GameActivity], which runs the game. The compatibility settings are the same family
 * ZalithLauncher2 exposes to players: renderer backend, resolution scaling, memory
 * allocation, touch controls, gyroscope, version isolation...
 */
class MainActivity : Activity() {

    private lateinit var settingsText: EditText
    private lateinit var graphicsApi: Spinner
    private lateinit var resolutionRatio: SeekBar
    private lateinit var ramAllocation: SeekBar
    private lateinit var touchControls: CheckBox
    private lateinit var gestureControl: CheckBox
    private lateinit var gyroscopeControl: CheckBox
    private lateinit var gamepadControl: CheckBox

    private val settingsFile = File(filesDir, "horizon-settings.json")

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        settingsText = findViewById(R.id.settings_text)
        graphicsApi = findViewById(R.id.graphics_api)
        resolutionRatio = findViewById(R.id.resolution_ratio)
        ramAllocation = findViewById(R.id.ram_allocation)
        touchControls = findViewById(R.id.touch_controls)
        gestureControl = findViewById(R.id.gesture_control)
        gyroscopeControl = findViewById(R.id.gyroscope_control)
        gamepadControl = findViewById(R.id.gamepad_control)

        loadSettings()
        setupGraphicsApi()
        setupResolution()
        setupRam()
        setupTouch()
        setupGyroscope()
        setupGamepad()
        setupPlayButton()
    }

    private fun loadSettings() {
        if (settingsFile.exists()) {
            settingsText.setText(settingsFile.readText())
        } else {
            settingsText.setText(defaultSettings())
        }
    }

    private fun saveSettings() {
        settingsFile.writeText(settingsText.text.toString())
    }

    private fun defaultSettings(): String = """
        {
          "graphics_api": "auto",
          "resolution_ratio": 100,
          "resolution_rule": "percentage",
          "game_full_screen": true,
          "sustained_performance": false,
          "request_max_refresh_rate": true,
          "version_isolation": true,
          "skip_game_integrity_check": false,
          "ram_allocation_mb": 2048,
          "show_fps": true,
          "fps_display_mode": "number",
          "touch_controls_enabled": true,
          "touch_look_mode": "slide",
          "touch_look_sensitivity": 100,
          "touch_move_sensitivity": 100,
          "hotbar_width": 500,
          "hotbar_height": 100,
          "hotbar_double_click": true,
          "hotbar_long_click": true,
          "hotbar_long_click_delay": 300,
          "gesture_control": true,
          "gesture_tap_action": "mouse_left",
          "gesture_long_press_action": "mouse_right",
          "gesture_long_press_delay": 300,
          "gyroscope_control": false,
          "gyroscope_sensitivity": 100,
          "gyroscope_sample_rate": 16,
          "gyroscope_smoothing": true,
          "gyroscope_smoothing_window": 4,
          "gyroscope_invert_x": false,
          "gyroscope_invert_y": false,
          "gamepad_control": true,
          "gamepad_input_mode": "mapped",
          "gamepad_dead_zone_scale": 100,
          "gamepad_mapping_config": "default",
          "language": "en",
          "gui_scale": 0,
          "max_fps": 260,
          "render_distance": 12,
          "simulation_distance": 12,
          "master_volume": 1.0,
          "show_log_automatic": false
        }
    """.trimIndent()

    private fun setupGraphicsApi() {
        val adapter = ArrayAdapter(this, android.R.layout.simple_spinner_dropdown_item,
            listOf("auto", "vulkan", "gles"))
        graphicsApi.adapter = adapter
        graphicsApi.onItemSelectedListener = object : AdapterView.OnItemSelectedListener {
            override fun onItemSelected(parent: AdapterView<*>?, view: View?, position: Int, id: Long) {
                setJsonField("graphics_api", adapter.getItem(position))
            }
            override fun onNothingSelected(parent: AdapterView<*>?) {}
        }
    }

    private fun setupResolution() {
        resolutionRatio.progress = 100
        resolutionRatio.setOnSeekBarChangeListener(object : SeekBar.OnSeekBarChangeListener {
            override fun onProgressChanged(seekBar: SeekBar?, progress: Int, fromUser: Boolean) {
                setJsonField("resolution_ratio", progress.toString())
            }
            override fun onStartTrackingTouch(seekBar: SeekBar?) {}
            override fun onStopTrackingTouch(seekBar: SeekBar?) {}
        })
    }

    private fun setupRam() {
        ramAllocation.progress = 2048
        ramAllocation.setOnSeekBarChangeListener(object : SeekBar.OnSeekBarChangeListener {
            override fun onProgressChanged(seekBar: SeekBar?, progress: Int, fromUser: Boolean) {
                setJsonField("ram_allocation_mb", progress.toString())
            }
            override fun onStartTrackingTouch(seekBar: SeekBar?) {}
            override fun onStopTrackingTouch(seekBar: SeekBar?) {}
        })
    }

    private fun setupTouch() {
        touchControls.isChecked = true
        touchControls.setOnCheckedChangeListener { _, isChecked ->
            setJsonField("touch_controls_enabled", isChecked.toString())
        }
        gestureControl.isChecked = true
        gestureControl.setOnCheckedChangeListener { _, isChecked ->
            setJsonField("gesture_control", isChecked.toString())
        }
    }

    private fun setupGyroscope() {
        gyroscopeControl.isChecked = false
        gyroscopeControl.setOnCheckedChangeListener { _, isChecked ->
            setJsonField("gyroscope_control", isChecked.toString())
        }
    }

    private fun setupGamepad() {
        gamepadControl.isChecked = true
        gamepadControl.setOnCheckedChangeListener { _, isChecked ->
            setJsonField("gamepad_control", isChecked.toString())
        }
    }

    private fun setupPlayButton() {
        findViewById<android.widget.Button>(R.id.play_button).setOnClickListener {
            saveSettings()
            startGame()
        }
    }

    /**
     * Write the launch request and start the game. The request tells the native side
     * where the game home is, what version to run, and which world or server to open.
     */
    private fun startGame() {
        val home = filesDir.resolve("horizon/versions/26.3")
        home.mkdirs()

        val config = """
            {
              "home": "${home.absolutePath}",
              "resources": "${home.absolutePath}",
              "version": "26.3",
              "config": "${settingsFile.absolutePath}",
              "world": "New World",
              "player": "Player",
              "width": 1920,
              "height": 1080
            }
        """.trimIndent()

        File(filesDir, "horizon-launch.json").writeText(config)

        val intent = Intent(this, GameActivity::class.java)
        startActivity(intent)
    }

    /** Update one field in the JSON text (simple string replacement). */
    private fun setJsonField(key: String, value: String) {
        val text = settingsText.text.toString()
        val newText = if (text.contains("\"$key\"")) {
            text.replace("\"$key\":[^,}]*", "\"$key\":$value")
        } else {
            text.replace("}", ",\"$key\":$value}")
        }
        settingsText.setText(newText)
    }
}
