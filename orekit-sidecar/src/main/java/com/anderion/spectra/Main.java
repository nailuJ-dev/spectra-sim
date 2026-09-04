package com.anderion.spectra;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.io.BufferedReader;
import java.io.File;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.io.StringReader;
import java.nio.charset.StandardCharsets;
import java.util.Properties;
import org.hipparchus.geometry.euclidean.threed.Vector3D;
import org.orekit.data.DataContext;
import org.orekit.data.DataProvidersManager;
import org.orekit.data.DataSource;
import org.orekit.data.DirectoryCrawler;
import org.orekit.files.ccsds.ndm.ParserBuilder;
import org.orekit.files.ccsds.ndm.odm.ocm.Ocm;
import org.orekit.files.ccsds.ndm.odm.ocm.TrajectoryStateHistory;
import org.orekit.files.ccsds.ndm.odm.oem.Oem;
import org.orekit.files.ccsds.ndm.odm.oem.OemSegment;
import org.orekit.frames.Frame;
import org.orekit.frames.FramesFactory;
import org.orekit.propagation.analytical.tle.TLE;
import org.orekit.propagation.analytical.tle.TLEPropagator;
import org.orekit.time.AbsoluteDate;
import org.orekit.time.TimeScalesFactory;
import org.orekit.utils.Constants;
import org.orekit.utils.IERSConventions;
import org.orekit.utils.PVCoordinates;
import org.orekit.utils.TimeStampedPVCoordinates;

public final class Main {
  private static final ObjectMapper JSON = new ObjectMapper();
  private static final String OREKIT_VERSION = readOrekitVersion();

  private Main() {}

  public static void main(String[] args) throws Exception {
    try {
      configureData();
    } catch (RuntimeException ex) {
      ObjectNode failure = base(false);
      failure.put("error", ex.getClass().getSimpleName() + ": " + ex.getMessage());
      String encoded = JSON.writeValueAsString(failure);
      System.out.println(encoded);
      System.out.flush();
      System.err.println(ex.getClass().getSimpleName() + ": " + ex.getMessage());
      System.exit(1);
      return;
    }

    try (BufferedReader reader =
        new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8))) {
      String line;
      while ((line = reader.readLine()) != null) {
        if (line.isBlank()) {
          continue;
        }
        ObjectNode response;
        try {
          response = handle(JSON.readTree(line));
        } catch (Exception ex) {
          response = base(false);
          response.put("error", ex.getClass().getSimpleName() + ": " + ex.getMessage());
        }
        System.out.println(JSON.writeValueAsString(response));
        System.out.flush();
      }
    }
  }

  private static void configureData() {
    String path = System.getenv("OREKIT_DATA_DIR");
    if (path == null || path.isBlank()) {
      throw new IllegalStateException(
          "OREKIT_DATA_DIR is required; network data providers are intentionally disabled");
    }
    File directory = new File(path);
    if (!directory.isDirectory()) {
      throw new IllegalStateException("OREKIT_DATA_DIR is not a directory: " + path);
    }
    DataProvidersManager manager = DataContext.getDefault().getDataProvidersManager();
    manager.clearProviders();
    manager.addProvider(new DirectoryCrawler(directory));
  }

  private static ObjectNode handle(JsonNode request) throws Exception {
    String op = requiredText(request, "op");
    return switch (op) {
      case "health" -> base(true).put("dataConfigured", true);
      case "propagate_tle" -> propagateTle(request);
      case "transform_state" -> transformState(request);
      case "parse_oem", "parse_oem_summary" -> parseOem(request, op.endsWith("_summary"));
      case "parse_ocm", "parse_ocm_summary" -> parseOcm(request, op.endsWith("_summary"));
      default -> throw new IllegalArgumentException("unsupported operation: " + op);
    };
  }

  private static ObjectNode propagateTle(JsonNode request) {
    TLE tle = new TLE(requiredText(request, "line1"), requiredText(request, "line2"));
    AbsoluteDate date =
        new AbsoluteDate(requiredText(request, "epochUtc"), TimeScalesFactory.getUTC());
    TLEPropagator propagator = TLEPropagator.selectExtrapolator(tle);
    Frame teme = FramesFactory.getTEME();
    PVCoordinates pv = propagator.getPVCoordinates(date, teme);
    ObjectNode response = base(true);
    ObjectNode result = response.putObject("result");
    result.put("frame", "TEME");
    result.put("epochUtc", utc(date));
    putVector(result.putArray("positionM"), pv.getPosition());
    putVector(result.putArray("velocityMPerS"), pv.getVelocity());
    putVector(result.putArray("accelerationMPerS2"), pv.getAcceleration());
    return response;
  }

  private static ObjectNode transformState(JsonNode request) {
    AbsoluteDate date =
        new AbsoluteDate(requiredText(request, "epochUtc"), TimeScalesFactory.getUTC());
    Frame source = frame(requiredText(request, "sourceFrame"));
    Frame target = frame(requiredText(request, "targetFrame"));
    Vector3D p = vector(request.get("positionM"));
    Vector3D v = vector(request.get("velocityMPerS"));
    PVCoordinates transformed =
        source
            .getTransformTo(target, date)
            .transformPVCoordinates(new PVCoordinates(p, v));
    ObjectNode response = base(true);
    ObjectNode result = response.putObject("result");
    result.put("frame", target.getName());
    result.put("epochUtc", utc(date));
    putVector(result.putArray("positionM"), transformed.getPosition());
    putVector(result.putArray("velocityMPerS"), transformed.getVelocity());
    putVector(result.putArray("accelerationMPerS2"), transformed.getAcceleration());
    return response;
  }

  private static ObjectNode parseOem(JsonNode request, boolean summaryOnly) {
    String content = requiredText(request, "content");
    ParserBuilder builder = parserBuilder();
    Oem oem = builder.buildOemParser().parse(dataSource("inline-oem", content));
    ObjectNode response = base(true);
    ObjectNode result = response.putObject("result");
    result.put("segmentCount", oem.getSegments().size());
    if (summaryOnly) {
      return response;
    }

    ArrayNode segments = result.putArray("segments");
    for (OemSegment segment : oem.getSegments()) {
      ObjectNode out = segments.addObject();
      out.put("objectName", segment.getMetadata().getObjectName());
      out.put("objectId", segment.getMetadata().getObjectID());
      out.put("frame", segment.getFrame().getName());
      out.put("startUtc", utc(segment.getStart()));
      out.put("stopUtc", utc(segment.getStop()));
      out.put("coordinateCount", segment.getCoordinates().size());
      ArrayNode states = out.putArray("states");
      for (TimeStampedPVCoordinates pv : segment.getCoordinates()) {
        putState(states.addObject(), pv);
      }
    }
    return response;
  }

  private static ObjectNode parseOcm(JsonNode request, boolean summaryOnly) {
    String content = requiredText(request, "content");
    ParserBuilder builder = parserBuilder();
    Ocm ocm = builder.buildOcmParser().parse(dataSource("inline-ocm", content));
    ObjectNode response = base(true);
    ObjectNode result = response.putObject("result");
    result.put("segmentCount", ocm.getSegments().size());
    if (summaryOnly) {
      return response;
    }

    ArrayNode segments = result.putArray("segments");
    for (var segment : ocm.getSegments()) {
      ObjectNode out = segments.addObject();
      out.put("objectName", segment.getMetadata().getObjectName());
      String objectId = segment.getMetadata().getInternationalDesignator();
      if (objectId == null || objectId.isBlank()) {
        objectId = segment.getMetadata().getObjectDesignator();
      }
      if (objectId != null && !objectId.isBlank()) {
        out.put("objectId", objectId);
      }
      ArrayNode blocks = out.putArray("trajectoryBlocks");
      for (TrajectoryStateHistory history : segment.getData().getTrajectoryBlocks()) {
        ObjectNode block = blocks.addObject();
        block.put("frame", history.getFrame().getName());
        block.put("trajectoryType", history.getMetadata().getTrajType().toString());
        block.put("startUtc", utc(history.getStart()));
        block.put("stopUtc", utc(history.getStop()));
        block.put("coordinateCount", history.getCoordinates().size());
        ArrayNode states = block.putArray("states");
        for (TimeStampedPVCoordinates pv : history.getCoordinates()) {
          putState(states.addObject(), pv);
        }
      }
    }
    return response;
  }

  private static ParserBuilder parserBuilder() {
    return new ParserBuilder()
        .withConventions(IERSConventions.IERS_2010)
        .withMu(Constants.EGM96_EARTH_MU);
  }

  private static void putState(ObjectNode node, TimeStampedPVCoordinates pv) {
    node.put("epochUtc", utc(pv.getDate()));
    putVector(node.putArray("positionM"), pv.getPosition());
    putVector(node.putArray("velocityMPerS"), pv.getVelocity());
    putVector(node.putArray("accelerationMPerS2"), pv.getAcceleration());
  }

  private static String utc(AbsoluteDate date) {
    return date.toString(TimeScalesFactory.getUTC());
  }

  private static DataSource dataSource(String name, String content) {
    return new DataSource(name, () -> new StringReader(content));
  }

  private static Frame frame(String name) {
    return switch (name.toUpperCase()) {
      case "GCRF", "ICRF" -> FramesFactory.getGCRF();
      case "TEME" -> FramesFactory.getTEME();
      case "EME2000", "J2000" -> FramesFactory.getEME2000();
      case "ITRF" -> FramesFactory.getITRF(IERSConventions.IERS_2010, false);
      default -> throw new IllegalArgumentException("unsupported frame: " + name);
    };
  }

  private static Vector3D vector(JsonNode node) {
    if (node == null || !node.isArray() || node.size() != 3) {
      throw new IllegalArgumentException("vector must contain exactly 3 values");
    }
    return new Vector3D(node.get(0).asDouble(), node.get(1).asDouble(), node.get(2).asDouble());
  }

  private static void putVector(ArrayNode array, Vector3D vector) {
    array.add(vector.getX());
    array.add(vector.getY());
    array.add(vector.getZ());
  }

  private static String requiredText(JsonNode request, String field) {
    JsonNode node = request.get(field);
    if (node == null || !node.isTextual() || node.asText().isBlank()) {
      throw new IllegalArgumentException("missing text field: " + field);
    }
    return node.asText();
  }

  private static String readOrekitVersion() {
    try (InputStream stream =
        Main.class.getResourceAsStream("/META-INF/maven/org.orekit/orekit/pom.properties")) {
      if (stream == null) {
        return "unknown";
      }
      Properties properties = new Properties();
      properties.load(stream);
      return properties.getProperty("version", "unknown");
    } catch (Exception ex) {
      return "unknown";
    }
  }

  private static ObjectNode base(boolean ok) {
    ObjectNode response = JSON.createObjectNode();
    response.put("ok", ok);
    response.put("orekitVersion", OREKIT_VERSION);
    return response;
  }
}
