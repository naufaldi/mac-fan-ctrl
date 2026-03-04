import { describe, expect, it } from "vitest";
import {
  formatSensorValue,
  partitionSensors,
  toFanRowData,
} from "./dashboardLayout";
import type { SensorData } from "./designTokens";

describe("dashboardLayout", () => {
  it("partitions sensors into fan and temperature groups", () => {
    const sensors: SensorData[] = [
      {
        id: "cpu",
        label: "CPU Package",
        value: 72,
        unit: "celsius",
        status: "warm",
      },
      {
        id: "fan-left",
        label: "Left Fan",
        value: 2450,
        unit: "rpm",
        status: "normal",
      },
    ];

    const partitions = partitionSensors(sensors);

    expect(partitions.temperatureSensors).toHaveLength(1);
    expect(partitions.fanSensors).toHaveLength(1);
    expect(partitions.fanSensors[0].id).toBe("fan-left");
  });

  it("maps fan row defaults for control scaffolding", () => {
    const fan: SensorData = {
      id: "fan-right",
      label: "Right Fan",
      value: 2300,
      unit: "rpm",
      status: "normal",
    };

    const row = toFanRowData(fan);

    expect(row.minRpm).toBe(1200);
    expect(row.maxRpm).toBe(6200);
    expect(row.controlMode).toBe("auto");
    expect(row.targetRpm).toBe(2300);
  });

  it("renders N/A for unavailable sensor values", () => {
    const sensor: SensorData = {
      id: "hdd",
      label: "HDD",
      value: null,
      unit: "celsius",
      status: "unknown",
    };

    expect(formatSensorValue(sensor)).toBe("N/A");
  });
});
