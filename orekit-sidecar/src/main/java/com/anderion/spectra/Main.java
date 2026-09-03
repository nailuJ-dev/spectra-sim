package com.anderion.spectra;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ArrayNode;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.io.BufferedReader;
import java.io.File;
import java.io.InputStreamReader;
import java.io.StringReader;
import java.nio.charset.StandardCharsets;
import org.hipparchus.geometry.euclidean.threed.Vector3D;
import org.orekit.data.DataContext;
import org.orekit.data.DataProvidersManager;
import org.orekit.data.DataSource;
import org.orekit.data.DirectoryCrawler;
import org.orekit.files.ccsds.ndm.ParserBuilder;
import org.orekit.files.ccsds.ndm.odm.ocm.Ocm;
import org.orekit.files.ccsds.ndm.odm.oem.Oem;
import org.orekit.frames.Frame;
import org.orekit.frames.FramesFactory;
import org.orekit.time.AbsoluteDate;
import org.orekit.time.TimeScalesFactory;
import org.orekit.utils.Constants;
import org.orekit.utils.IERSConventions;
import org.orekit.utils.PVCoordinates;
import org.orekit.propagation.analytical.tle.TLE;
import org.orekit.propagation.analytical.tle.TLEPropagator;

public final class Main {
  private static final ObjectMapper JSON = new ObjectMapper();

  private Main() {}

  public static void main(String[] args) throws Exception {
    configureData();
    try (BufferedReader reader = new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8))) {
      String line;
      while ((line = reader.readLine()) != null) {
        if (line.isBlank()) continue;
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
      throw new IllegalStateException("OREKIT_DATA_DIR is required; network data providers are intentionally disabled");
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
      case "parse_oem_summary" -> parseOemSummary(request);
      case "parse_ocm_summary" -> parseOcmSummary(request);
      default -> throw new IllegalArgumentException("unsupported operation: " + op);
    };
  }

  private static ObjectNode propagateTle(JsonNode request) {
    TLE tle = new TLE(requiredText(request, "line1"), requiredText(request, "line2"));
    AbsoluteDate date = new AbsoluteDate(requiredText(request, "epochUtc"), TimeScalesFactory.getUTC());
    TLEPropagator propagator = TLEPropagator.selectExtrapolator(tle);
    Frame teme = FramesFactory.getTEME();
    PVCoordinates pv = propagator.getPVCoordinates(date, teme);
    ObjectNode response = base(true);
    ObjectNode result = response.putObject("result");
    result.put("frame", "TEME");
    result.put("epochUtc", date.toString(TimeScalesFactory.getUTC()));
    putVector(result.putArray("positionM"), pv.getPosition());
    putVector(result.putArray("velocityMPerS"), pv.getVelocity());
    return response;
  }

  private static ObjectNode transformState(JsonNode request) {
    AbsoluteDate date = new AbsoluteDate(requiredText(request, "epochUtc"), TimeScalesFactory.getUTC());
    Frame source = frame(requiredText(request, "sourceFrame"));
    Frame target = frame(requiredText(request, "targetFrame"));
    Vector3D p = vector(request.get("positionM"));
    Vector3D v = vector(request.get("velocityMPerS"));
    PVCoordinates transformed = source.getTransformTo(target, date).transformPVCoordinates(new PVCoordinates(p, v));
    ObjectNode response = base(true);
    ObjectNode result = response.putObject("result");
    result.put("frame", target.getName());
    putVector(result.putArray("positionM"), transformed.getPosition());
    putVector(result.putArray("velocityMPerS"), transformed.getVelocity());
    return response;
  }

  private static ObjectNode parseOemSummary(JsonNode request) {
    String content = requiredText(request, "content");
    ParserBuilder builder = new ParserBuilder()
        .withConventions(IERSConventions.IERS_2010)
        .withMu(Constants.EGM96_EARTH_MU);
    Oem oem = builder.buildOemParser().parse(dataSource("inline-oem", content));
    ObjectNode response = base(true);
    ObjectNode result = response.putObject("result");
    result.put("segmentCount", oem.getSegments().size());
    return response;
  }

  private static ObjectNode parseOcmSummary(JsonNode request) {
    String content = requiredText(request, "content");
    ParserBuilder builder = new ParserBuilder()
        .withConventions(IERSConventions.IERS_2010)
        .withMu(Constants.EGM96_EARTH_MU);
    Ocm ocm = builder.buildOcmParser().parse(dataSource("inline-ocm", content));
    ObjectNode response = base(true);
    ObjectNode result = response.putObject("result");
    result.put("segmentCount", ocm.getSegments().size());
    return response;
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
    array.add(vector.getX()); array.add(vector.getY()); array.add(vector.getZ());
  }

  private static String requiredText(JsonNode request, String field) {
    JsonNode node = request.get(field);
    if (node == null || !node.isTextual() || node.asText().isBlank()) {
      throw new IllegalArgumentException("missing text field: " + field);
    }
    return node.asText();
  }

  private static ObjectNode base(boolean ok) {
    ObjectNode response = JSON.createObjectNode();
    response.put("ok", ok);
    response.put("orekitVersion", "13.1.8");
    return response;
  }
}
